use steam_api::suggest::{get_suggestions, Suggestion};
use teloxide::{
    prelude::*,
    types::{
        InlineKeyboardButton, InlineKeyboardMarkup, InlineQueryResult, InlineQueryResultArticle,
        InputMessageContent, InputMessageContentText,
    },
    Bot, RequestError,
};
use tokio::task::JoinSet;

use crate::steam_api::{app_hover::get_app_hover_details, dlc::get_dlcs};

mod steam_api;
mod utils;

async fn build_messages(query_term: &str) -> anyhow::Result<Vec<(Suggestion, String)>> {
    use std::fmt::Write;

    let mut results = Vec::new();
    let mut join_set = JoinSet::new();

    for suggestion in get_suggestions(query_term).await? {
        join_set.spawn(async move {
            let Suggestion { name, price, .. } = &suggestion;
            let app_id = suggestion.id;

            let (app_hover_details, dlcs) =
                tokio::try_join!(get_app_hover_details(app_id), get_dlcs(app_id))?;

            let maybe_platforms = dlcs.dlcs.first().map(|dlc| &dlc.platforms);

            let mut body = String::with_capacity(2048);

            writeln!(
                body,
                "🎮 [{name}](https://store.steampowered.com/app/{app_id}/) - {price}\n"
            )?;
            if let Some(platforms) = maybe_platforms {
                writeln!(body, "🖥 Plataformas suportadas\n{platforms}\n")?;
            }
            writeln!(
                body,
                "💬 Descrição\n {}\n\n🏆 Avaliações\n{} ({} avaliações)\n\n{}\n",
                app_hover_details.description,
                app_hover_details.review_summary.review_summary,
                app_hover_details.review_summary.review_count,
                app_hover_details.release_date,
            )?;
            writeln!(body, "💸 {dlcs}")?;

            Ok((suggestion, body)) as anyhow::Result<(Suggestion, String)>
        });
    }

    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(Ok((suggestion, body))) => results.push((suggestion, body)),
            Ok(Err(err)) => tracing::error!("Problem fetching from SteamAPI: {err}"),
            Err(err) => {
                tracing::error!("Problem joining future: {err}")
            }
        }
    }

    Ok(results)
}

#[tokio::main]
async fn main() {
    start_tracing();

    let bot = Bot::from_env();

    let handler = Update::filter_inline_query().branch(dptree::endpoint(
        |bot: Bot, q: InlineQuery| async move {
            let query_term = &q.query;

            let Ok(messages) = build_messages(query_term).await else {
                eprintln!("Failed to get Suggestions, ignoring query");
                return Err(RequestError::RetryAfter(std::time::Duration::from_secs(5)));
            };

            let results: Vec<_> = messages
                .into_iter()
                .enumerate()
                .map(|(idx, (suggestion, body))| {
                    let title = format!("{} - {}", suggestion.name, suggestion.price);
                    InlineQueryResultArticle::new(
                        format!("{idx}"),
                        // What the user will actually see
                        &title,
                        // What message will be sent when clicked/tapped
                        InputMessageContent::Text(
                            #[allow(deprecated)]
                            InputMessageContentText::new(body)
                                .parse_mode(teloxide::types::ParseMode::Markdown),
                        ),
                    )
                    .title(title)
                    .thumb_url(suggestion.img.parse().unwrap())
                    .reply_markup({
                        let protondb = format!("https://protondb.com/app/{}/", suggestion.id)
                            .parse()
                            .unwrap();
                        let steamdb = format!("https://steamdb.info/app/{}/", suggestion.id)
                            .parse()
                            .unwrap();

                        let protondb = InlineKeyboardButton::url("ProtonDB", protondb);
                        let steamdb = InlineKeyboardButton::url("SteamDB", steamdb);

                        InlineKeyboardMarkup::default().append_row([protondb, steamdb])
                    })
                })
                .map(InlineQueryResult::Article)
                .collect();

            let response = bot.answer_inline_query(&q.id, results).send().await;
            if let Err(err) = response {
                tracing::error!("Error in handler: {:?}", err);
            }
            respond(())
        },
    ));

    Dispatcher::builder(bot, handler).build().dispatch().await;

    // TODO: price history
    // let suggestions = dbg!(get_suggestions("gris").await.unwrap());
    //
    // for suggestion in suggestions {
    //     fetch_game_data(suggestion.id).await.unwrap();
    // }
}

fn start_tracing() {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();
}
