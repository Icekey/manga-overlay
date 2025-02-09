use anyhow::{anyhow, Result};
use itertools::Itertools;

pub async fn translate(jpn_text: &str) -> Result<String> {
    let url = "https://translate.google.com/m?sl=ja&tl=en&hl=en";
    let response = reqwest::get(format!("{url}&q={jpn_text}")).await?;
    let body = response.text().await?;

    let document = scraper::Html::parse_document(&body);

    let selector = scraper::Selector::parse("div.result-container")
        .map_err(|_| anyhow!("div.result-container selector not found"))?;
    let translation = document.select(&selector).map(|x| x.inner_html()).join("");
    Ok(translation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::info;

    #[tokio::test]
    async fn test_request_google() {
        let body = translate("今 いま 私 わたし\n は 東京 とうきょう に 住 す んでいるので")
            .await
            .unwrap();
        info!("{}", body);
    }
}
