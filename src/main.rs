use std::{sync::Arc, time::Instant};

use frankenstein::{
    AllowedUpdate, AnswerInlineQueryParams, AsyncApi, AsyncTelegramApi, GetUpdatesParams, InlineKeyboardButton, InlineKeyboardMarkup, InlineQueryResult, InlineQueryResultArticle, InputMessageContent, InputTextMessageContent, ParseMode, UpdateContent
};
use steam_api::suggest::{get_suggestions, Suggestion};

use tokio::task::JoinSet;

use crate::steam_api::{
    app_hover::get_app_hover_details, dlc::get_dlcs, protondb::get_proton_compatibility,
    steam_deck::get_steam_deck_compatibility,
};

lazy_static::lazy_static! {
    static ref HTTP_CLIENT: reqwest::Client = reqwest::Client::new();
}

mod steam_api;
mod utils;

async fn build_messages(query_term: &str) -> anyhow::Result<Vec<(Suggestion, String)>> {
    use std::fmt::Write;

    let suggestions = {
        let start = Instant::now();
        let suggestions = get_suggestions(query_term).await?;
        tracing::info!(
            "Fetched suggestions for query '{query_term}' in {}ms",
            start.elapsed().as_millis()
        );

        suggestions
    };

    let mut results = Vec::with_capacity(suggestions.len());
    let mut join_set = JoinSet::new();

    let start = Instant::now();

    for suggestion in suggestions {
        join_set.spawn(async move {
            let Suggestion { name, price, .. } = &suggestion;
            let app_id = suggestion.id;

            let (app_hover_details, dlcs, maybe_deck_compat, proton_compat) =
                tokio::try_join!(
                    get_app_hover_details(app_id),
                    get_dlcs(app_id),
                    get_steam_deck_compatibility(app_id),
                    get_proton_compatibility(app_id)
                )?;

            let maybe_platforms = dlcs.dlcs.first().map(|dlc| &dlc.platforms);

            let mut body = String::with_capacity(2048);

            writeln!(
                body,
                "[{name}](https://cdn.akamai.steamstatic.com/steam/apps/{app_id}/header.jpg) - {price}\n"
            )?;
            if let Some(platforms) = maybe_platforms {
                writeln!(body, "*Plataformas suportadas*: {platforms}\n")?;
            }
            writeln!(body, "*Status no ProtonDB*: {} ({} relatórios)", convert_proton_tier(&proton_compat.trending_tier), proton_compat.total)?;
            if let Some(deck_compat) = maybe_deck_compat {
                writeln!(body, "*Compatibilidade com o Steam Deck*: {deck_compat}\n")?;
            }
            writeln!(
                body,
                "*Descrição*\n{}\n\n*Avaliações*: {} ({} avaliações)\n\n*Lançamento*: {}\n",
                app_hover_details.description,
                app_hover_details.review_summary.review_summary,
                app_hover_details.review_summary.review_count,
                get_release_date(&app_hover_details.release_date),
            )?;
            writeln!(body, "{dlcs}")?;

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

    tracing::info!(
        "Built bodies for all messages for query {query_term} in {}ms",
        start.elapsed().as_millis()
    );

    Ok(results)
}

#[tokio::main(worker_threads = 16)]
async fn main() {
    start_tracing();

    let api = AsyncApi::new("6837644181:AAGG9R6NmT3Rjf2v4TMd74ky0WzBRE0B2wE");
    let api = Arc::new(api);

    let update_params_builder =
        GetUpdatesParams::builder().allowed_updates([AllowedUpdate::InlineQuery]);
    let mut update_params = update_params_builder.clone().build();

    loop {
        let result = api.get_updates(&update_params).await;

        match result {
            Ok(response) => {
                for update in response.result {
                    if let UpdateContent::InlineQuery(inline_query) = update.content {
                        let started_at = Instant::now();
                        let now = Instant::now();
                        let Ok(messages) = build_messages(&inline_query.query).await else {
                            eprintln!("Failed to get Suggestions, ignoring query");
                            continue;
                        };
                        tracing::info!("Built data for query {} in {}ms", inline_query.query, now.elapsed().as_millis());
                        let now = Instant::now();

                        let replies: Vec<_> = messages
                            .into_iter()
                            .enumerate()
                            .map(|(idx, (suggestion, body))| {
                                let title = format!("{} - {}", suggestion.name, suggestion.price);

                                InlineQueryResult::Article(
                                    InlineQueryResultArticle::builder()
                                        .id(idx.to_string())
                                        .title(title)
                                        .thumbnail_url(suggestion.img.parse::<String>().unwrap())
                                        .input_message_content(InputMessageContent::Text(
                                            #[allow(deprecated)]
                                            InputTextMessageContent::builder()
                                                .message_text(body)
                                                .parse_mode(ParseMode::Markdown)
                                                .build(),
                                        ))
                                        .reply_markup({
                                            let protondb: String = format!(
                                                "https://protondb.com/app/{}/",
                                                suggestion.id
                                            )
                                            .parse()
                                            .unwrap();
                                            let steamdb: String = format!(
                                                "https://steamdb.info/app/{}/",
                                                suggestion.id
                                            )
                                            .parse()
                                            .unwrap();
                                            let steam: String = format!(
                                                "https://store.steampowered.com/app/{}/",
                                                suggestion.id
                                            )
                                            .parse()
                                            .unwrap();

                                            let protondb = InlineKeyboardButton::builder()
                                                .url(protondb)
                                                .text("ProtonDB")
                                                .build();
                                            let steamdb = InlineKeyboardButton::builder()
                                                .url(steamdb)
                                                .text("SteamDB")
                                                .build();
                                            let steam = InlineKeyboardButton::builder()
                                                .text("Página na Steam")
                                                .url(steam)
                                                .build();

                                            InlineKeyboardMarkup::builder()
                                                .inline_keyboard(vec![
                                                    vec![protondb, steamdb],
                                                    vec![steam],
                                                ])
                                                .build()
                                        })
                                        .build(),
                                )
                            })
                            .collect();

                        tracing::info!("Built inline query responses in {}ms", now.elapsed().as_millis());
                        let now = Instant::now();

                        let answer = AnswerInlineQueryParams::builder()
                            .inline_query_id(inline_query.id)
                            .results(replies)
                            .build();

                        if let Err(err) = api.answer_inline_query(&answer).await {
                            tracing::error!("Failed to response inline query: {err}")
                        }
                        tracing::info!("Sent response in {}ms", now.elapsed().as_millis());
                        tracing::info!("Whole process took {}ms", started_at.elapsed().as_millis());

                        update_params = update_params_builder
                            .clone()
                            .offset(update.update_id + 1)
                            .build();
                    }
                }
            }
            Err(err) => {
                tracing::error!("Failed to get inline query: {err}")
            }
        }
    }
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

fn get_release_date(input: &str) -> &str {
    input
        .rsplit_once(' ')
        .map(|(_before, after)| after)
        .unwrap_or(input)
}

fn convert_proton_tier(input: &str) -> &str {
    match input {
        "platinum" => "Platina",
        "gold" => "Ouro",
        "silver" => "Prata",
        "borked" => "Quebrado",
        "bronze" => "Bronze",
        other => other,
    }
}
