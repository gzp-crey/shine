use futures::stream::StreamExt;
use gremlin_client::{aio::GremlinClient, FromGValue, GremlinError, ToGValue};

pub async fn query_vec<R>(
    client: &GremlinClient,
    script: &str,
    params: &[(&str, &dyn ToGValue)],
) -> Result<Vec<R>, GremlinError>
where
    R: FromGValue,
{
    let vec_of_results: Vec<_> = client
        .execute(script, params)
        .await?
        .map(|r| r?.take::<R>()) // map from Result<GValue> to Result<R>
        .collect()
        .await;
    vec_of_results.into_iter().collect()
}

pub async fn query_value<R>(
    client: &GremlinClient,
    script: &str,
    params: &[(&str, &dyn ToGValue)],
) -> Result<R, GremlinError>
where
    R: FromGValue,
{
    let vec_of_results: Vec<_> = client
        .execute(script, params)
        .await?
        .map(|r| r?.take::<R>()) // map from Result<GValue> to Result<R>
        .collect()
        .await;

    if vec_of_results.len() != 1 {
        Err(GremlinError::Generic(format!(
            "Invalid response count: {}",
            vec_of_results.len()
        )))
    } else {
        vec_of_results
            .into_iter()
            .next()
            .ok_or(GremlinError::Generic(format!("Missing response")))?
    }
}

pub async fn query_opt_value<R>(
    client: &GremlinClient,
    script: &str,
    params: &[(&str, &dyn ToGValue)],
) -> Result<Option<R>, GremlinError>
where
    R: FromGValue,
{
    let vec_of_results: Vec<_> = client
        .execute(script, params)
        .await?
        .map(|r| r?.take::<R>()) // map from Result<GValue> to Result<R>
        .collect()
        .await;

    if vec_of_results.len() > 1 {
        Err(GremlinError::Generic(format!(
            "Invalid response count: {}",
            vec_of_results.len()
        )))
    } else {
        match vec_of_results.into_iter().next() {
            None => Ok(None),
            Some(Err(e)) => Err(e),
            Some(Ok(r)) => Ok(Some(r)),
        }
    }
}
