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
    bl.insert(Url::parse("https://darmarit.org/searx/").unwrap());
    bl.insert(Url::parse("https://dynabyte.ca/").unwrap());
    bl.insert(Url::parse("https://engo.mint.lgbt/").unwrap());
    bl.insert(Url::parse("https://haku.lelux.fi/").unwrap());
    bl.insert(Url::parse("https://methylcraft.com/search/").unwrap());
    bl.insert(Url::parse("https://nibblehole.com/").unwrap());
    bl.insert(Url::parse("https://privatesearch.app/").unwrap());
    bl.insert(Url::parse("https://recherche.catmargue.org/").unwrap());
    bl.insert(Url::parse("https://spot.ecloud.global/").unwrap());
    bl.insert(Url::parse("https://search.disroot.org/").unwrap());
    bl.insert(Url::parse("https://search.ethibox.fr/").unwrap());
    bl.insert(Url::parse("https://search.jigsaw-security.com/").unwrap());
    bl.insert(Url::parse("https://search.jpope.org/").unwrap());
    bl.insert(Url::parse("https://search.mdosch.de/").unwrap());
    bl.insert(Url::parse("https://search.modalogi.com/").unwrap());
    bl.insert(Url::parse("https://search.snopyta.org/").unwrap());
    bl.insert(Url::parse("https://search.st8.at/").unwrap());
    bl.insert(Url::parse("https://search.stinpriza.org/").unwrap());
    bl.insert(Url::parse("https://searx.ch/").unwrap());
    bl.insert(Url::parse("https://searx.be/").unwrap());
    bl.insert(Url::parse("https://searx.decatec.de/").unwrap());
    bl.insert(Url::parse("https://searx.devol.it/").unwrap());
    bl.insert(Url::parse("https://searx.dresden.network/").unwrap());
    bl.insert(Url::parse("https://searx.everdot.org/").unwrap());
    bl.insert(Url::parse("https://searx.fmac.xyz/").unwrap());
    bl.insert(Url::parse("https://searx.fossencdi.org/").unwrap());
    bl.insert(Url::parse("https://searx.gnu.style/").unwrap());
    bl.insert(Url::parse("https://searx.hardwired.link/").unwrap());
    bl.insert(Url::parse("https://searx.ir/").unwrap());
    bl.insert(Url::parse("https://searx.laquadrature.net/").unwrap());
    bl.insert(Url::parse("https://searx.lavatech.top/").unwrap());
    bl.insert(Url::parse("https://searx.lelux.fi/").unwrap());
    bl.insert(Url::parse("https://searx.likkle.monster/").unwrap());
    bl.insert(Url::parse("https://searx.lnode.net/").unwrap());
    bl.insert(Url::parse("https://searx.mastodontech.de/").unwrap());
    bl.insert(Url::parse("https://searx.mxchange.org/").unwrap());
    bl.insert(Url::parse("https://searx.nakhan.net/").unwrap());
    bl.insert(Url::parse("https://searx.netzspielplatz.de/").unwrap());
    bl.insert(Url::parse("https://searx.nevrlands.de/").unwrap());
    bl.insert(Url::parse("https://searx.nixnet.services/").unwrap());
    bl.insert(Url::parse("https://searx.org/").unwrap());
    bl.insert(Url::parse("https://searx.openhoofd.nl/").unwrap());
    bl.insert(Url::parse("https://searx.openpandora.org/").unwrap());
    bl.insert(Url::parse("https://searx.operationtulip.com/").unwrap());
    bl.insert(Url::parse("https://searx.ouahpiti.info/").unwrap());
    bl.insert(Url::parse("https://searx.pwoss.org/").unwrap());
    bl.insert(Url::parse("https://searx.roflcopter.fr/").unwrap());
    bl.insert(Url::parse("https://searx.roughs.ru/").unwrap());
    bl.insert(Url::parse("https://searx.run/").unwrap());
    bl.insert(Url::parse("https://searx.simonoener.com/").unwrap());
    bl.insert(Url::parse("https://searx.slash-dev.de/").unwrap());
    bl.insert(Url::parse("https://searx.solusar.de/").unwrap());
    bl.insert(Url::parse("https://searx.sunless.cloud/").unwrap());
    bl.insert(Url::parse("https://searx.thegreenwebfoundation.org/").unwrap());
    bl.insert(Url::parse("https://searx.tunkki.xyz/searx/").unwrap());
    bl.insert(Url::parse("https://searx.tyil.nl/").unwrap());
    bl.insert(Url::parse("https://searx.xyz/").unwrap());
    bl.insert(Url::parse("https://suche.dasnetzundich.de/").unwrap());
    bl.insert(Url::parse("https://timdor.noip.me/searx/").unwrap());
    bl.insert(Url::parse("https://www.perfectpixel.de/searx/").unwrap());
    bl.insert(Url::parse("https://www.searxs.eu/").unwrap());
    bl.insert(Url::parse("https://zoek.anchel.nl/").unwrap());

    bl
});

#[derive(Deserialize)]
struct ServerList {
    instances: HashMap<Url, Instance>,
}

#[derive(Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
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
