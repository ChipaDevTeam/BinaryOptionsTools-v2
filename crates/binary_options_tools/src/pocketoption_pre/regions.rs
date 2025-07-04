use binary_options_tools_macros::RegionImpl;

use crate::pocketoption::{
    error::PocketResult,
    utils::location::{calculate_distance, get_public_ip, get_user_location},
};

#[derive(RegionImpl)]
#[region(path = "regions.json")]
pub struct Regions;

impl Regions {
    async fn get_closest_server(&self, ip_address: &str) -> PocketResult<(&str, f64)> {
        let user_location = get_user_location(ip_address).await?;

        let mut closest = ("", f64::INFINITY);
        Self::regions().iter().for_each(|(server, lat, lon)| {
            let distance = calculate_distance(user_location.0, user_location.1, *lat, *lon);
            if distance < closest.1 {
                closest = (*server, distance)
            }
        });
        Ok(closest)
    }

    async fn sort_servers(&self, ip_address: &str) -> PocketResult<Vec<&str>> {
        let user_location = get_user_location(ip_address).await?;
        let mut distances = Self::regions()
            .iter()
            .map(|(server, lat, lon)| {
                (
                    *server,
                    calculate_distance(user_location.0, user_location.1, *lat, *lon),
                )
            })
            .collect::<Vec<(&str, f64)>>();
        distances.sort_by(|(_, a), (_, b)| b.total_cmp(a));
        Ok(distances.into_iter().map(|(s, _)| s).collect())
    }

    pub async fn get_server(&self) -> PocketResult<&str> {
        let ip = get_public_ip().await?;
        let server = self.get_closest_server(&ip).await?;
        Ok(server.0)
    }

    pub async fn get_servers(&self) -> PocketResult<Vec<&str>> {
        let ip = get_public_ip().await?;
        self.sort_servers(&ip).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_closest_server() -> anyhow::Result<()> {
        // let ip = get_public_ip().await?;
        let server = Regions.get_server().await?;
        dbg!(server);
        Ok(())
    }
}
