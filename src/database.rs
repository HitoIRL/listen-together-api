use poem::{async_trait, FromRequest, Request, RequestBody, Result};
use redis::aio::ConnectionManager;

pub struct Database(pub ConnectionManager);

#[async_trait]
impl<'a> FromRequest<'a> for Database {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        let con = req.extensions().get::<ConnectionManager>().unwrap();
        Ok(Database(con.clone()))
    }
}
