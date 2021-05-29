use itertools::Itertools;
use octocrab::{
    self,
    models::{
        repos::{Asset, Release},
        AssetId,
    },
    repos::ReleasesHandler,
    Error,
};
use regex::Regex;

#[derive(Debug, Clone)]
pub enum ReleaseFinder {
    Latest,
    ByTag(String),
    ByRegex(Regex),
}

impl ReleaseFinder {
    async fn find(&self, rels: ReleasesHandler<'_, '_>) -> Result<Option<Release>, Error> {
        match self {
            ReleaseFinder::Latest => rels.get_latest().await.map(Some),
            // TODO: handle missing release in the by tag
            ReleaseFinder::ByTag(tag) => rels.get_by_tag(&tag).await.map(Some),
            ReleaseFinder::ByRegex(re) => {
                let mut current_page = rels.list().per_page(100).page(0u32).send().await?;
                let mut prs = current_page.take_items();

                let inst = octocrab::instance();
                while let Ok(Some(mut new_page)) = inst.get_page(&current_page.next).await {
                    prs.extend(new_page.take_items());

                    for page in prs.drain(..) {
                        if re.is_match(&page.tag_name) {
                            return Ok(Some(page));
                        }
                    }
                }

                Ok(None)
                // Ok(prs.into_iter().findl(|x| re.is_match(&x.tag_name)))
            }
        }
    }
    fn find_from(&self, rels: Vec<Release>) -> Vec<Release> {
        match self {
            ReleaseFinder::Latest => vec![rels[0].clone()],
            ReleaseFinder::ByTag(tag) => rels
                .iter()
                .cloned()
                .filter(|rel| &rel.tag_name == tag)
                .collect_vec(),
            ReleaseFinder::ByRegex(re) => rels
                .iter()
                .cloned()
                .filter(|rel| re.is_match(&rel.tag_name))
                .collect_vec(),
        }
    }
}

impl Default for ReleaseFinder {
    fn default() -> Self {
        Self::Latest
    }
}

#[derive(Debug, Clone)]
pub enum AssetFinder {
    ByRegex(ReleaseFinder, Regex),
    ById(AssetId),
}

impl AssetFinder {
    async fn find(&self, rels: ReleasesHandler<'_, '_>) -> Result<Option<Asset>, Error> {
        Ok(match self {
            AssetFinder::ByRegex(relfin, re) => {
                let rel = relfin.find(rels).await?;
                eprintln!("search through {:#?}", rel);
                rel.and_then(|rel| rel.assets.into_iter().find(|x| re.is_match(&x.name)))
            }
            // TODO: handle missing asset in the by id (convert to option)
            &AssetFinder::ById(id) => Some(rels.get_asset(id).await?),
        })
    }
}

pub fn find_release_from(re: &Regex, assets: &[Release]) -> Vec<Release> {
    assets
        .iter()
        .filter(|rel| re.is_match(&rel.tag_name))
        .cloned()
        .collect_vec()
}
pub fn find_asset_from(re: &Regex, assets: &[Asset]) -> Vec<Asset> {
    assets
        .iter()
        .filter(|ass| re.is_match(&ass.name))
        .cloned()
        .collect_vec()
}

pub async fn list_releases_page(
    user: &str,
    repo: &str,
    page: u32,
    per: u8,
) -> Result<Vec<Release>, Error> {
    let inst = octocrab::instance();
    let repos = inst.repos(user, repo);
    let rels = repos.releases();
    Ok(rels
        .list()
        .per_page(per.min(100))
        .page(page)
        .send()
        .await?
        .take_items())
}
pub async fn list_releases(user: &str, repo: &str) -> Result<Vec<Release>, Error> {
    let inst = octocrab::instance();
    let repos = inst.repos(user, repo);
    let rels = repos.releases();
    let mut current_page = rels.list().per_page(100).page(0u32).send().await?;
    let mut prs = current_page.take_items();
    while let Ok(Some(mut new_page)) = inst.get_page(&current_page.next).await {
        prs.extend(new_page.take_items());
    }
    Ok(prs)
}
pub async fn find_release(
    user: &str,
    repo: &str,
    find: ReleaseFinder,
) -> Result<Option<Release>, Error> {
    let inst = octocrab::instance();
    let repos = inst.repos(user, repo);
    let rels = repos.releases();

    find.find(rels).await
}
pub async fn find_asset(user: &str, repo: &str, find: AssetFinder) -> Result<Option<Asset>, Error> {
    let inst = octocrab::instance();
    let repos = inst.repos(user, repo);
    let rels = repos.releases();

    find.find(rels).await
}

pub async fn download_asset(asset: Asset) -> Result<Vec<u8>, Error> {
    todo!("Choose how to downlod")
}
