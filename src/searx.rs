//! Interfacing with Searx instances.

use anyhow::{
    Context,
    Result,
};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::{
    HashMap,
    HashSet,
};
use url::Url;

const SERVERLIST_URL: &str = "https://searx.space/data/instances.json";

#[allow(clippy::unwrap_used)]
static BLACKLIST: Lazy<HashSet<Url>> = Lazy::new(|| {
    let mut bl = HashSet::new();

    bl.insert(Url::parse("https://azkware.net/yunohost/sso/?r=aHR0cHM6Ly9zZWFyY2guYXprd2FyZS5uZXQv/").unwrap());
    bl.insert(Url::parse("https://engo.mint.lgbt/").unwrap());
    bl.insert(Url::parse("https://methylcraft.com/").unwrap());
    bl.insert(Url::parse("https://nibblehole.com/").unwrap());
    bl.insert(Url::parse("https://recherche.catmargue.org/").unwrap());
    bl.insert(Url::parse("https://search.jigsaw-security.com/").unwrap());
    bl.insert(Url::parse("https://search.jpope.org/").unwrap());
    bl.insert(Url::parse("https://search.snopyta.org/").unwrap());
    bl.insert(Url::parse("https://search.st8.at/").unwrap());
    bl.insert(Url::parse("https://search.stinpriza.org/").unwrap());
    bl.insert(Url::parse("https://searx.ch/").unwrap());
    bl.insert(Url::parse("https://searx.decatec.de/").unwrap());
    bl.insert(Url::parse("https://searx.devol.it/").unwrap());
    bl.insert(Url::parse("https://searx.dresden.network/").unwrap());
    bl.insert(Url::parse("https://searx.everdot.org/").unwrap());
    bl.insert(Url::parse("https://searx.fossencdi.org/").unwrap());
    bl.insert(Url::parse("https://searx.hardwired.link/").unwrap());
    bl.insert(Url::parse("https://searx.laquadrature.net/").unwrap());
    bl.insert(Url::parse("https://searx.lavatech.top/").unwrap());
    bl.insert(Url::parse("https://searx.lnode.net/").unwrap());
    bl.insert(Url::parse("https://searx.mastodontech.de/").unwrap());
    bl.insert(Url::parse("https://searx.mxchange.org/").unwrap());
    bl.insert(Url::parse("https://searx.nakhan.net/").unwrap());
    bl.insert(Url::parse("https://searx.netzspielplatz.de/").unwrap());
    bl.insert(Url::parse("https://searx.nevrlands.de/").unwrap());
    bl.insert(Url::parse("https://searx.nixnet.services/").unwrap());
    bl.insert(Url::parse("https://searx.openhoofd.nl/").unwrap());
    bl.insert(Url::parse("https://searx.operationtulip.com/").unwrap());
    bl.insert(Url::parse("https://searx.roflcopter.fr/").unwrap());
    bl.insert(Url::parse("https://searx.slash-dev.de/").unwrap());
    bl.insert(Url::parse("https://searx.thegreenwebfoundation.org/").unwrap());
    bl.insert(Url::parse("https://searx.tunkki.xyz/searx/").unwrap());
    bl.insert(Url::parse("https://suche.dasnetzundich.de/").unwrap());
    bl.insert(Url::parse("https://timdor.noip.me/searx/").unwrap());
    bl.insert(Url::parse("https://www.perfectpixel.de/searx/").unwrap());

    bl
});

#[derive(Deserialize)]
struct ServerList {
    instances: HashMap<Url, Instance>,
}

#[serde(rename_all = "lowercase")]
#[derive(Deserialize, Eq, PartialEq)]
enum NetworkType {
    Normal,
    Tor,
}

#[derive(Deserialize)]
struct Instance {
    network_type: NetworkType,
    http: HttpStatus,
}

#[derive(Deserialize)]
struct HttpStatus {
    status_code: Option<u16>,
}

pub(crate) fn fetch_serverlist() -> Result<Vec<Url>> {
    Ok(ureq::get(SERVERLIST_URL)
        .call()
        .context("failed to fetch Searx serverlist")?
        .into_json::<ServerList>()
        .context("failed to decode Searx serverlist")?
        .instances
        .into_iter()
        .filter_map(|(url, instance)| {
            (instance.network_type == NetworkType::Normal
                && instance.http.status_code.unwrap_or(0) == 200
                && !BLACKLIST.contains(&url))
            .then(|| url.join("search").expect("valid search URL"))
        })
        .collect())
}
