use failure::Error;
use hostname;
use response::errors;
use rocket::State;
use rocket::http::{Header, Status};
use rocket::request::Request;
use rocket::response::{Responder, Response};

use backend as bh;
use grpc::backend;
use types::Layer;

#[derive(Debug, Serialize)]
pub struct UploadInfo {
    uuid: String,
    name: String,
    repo: String,
    range: (u32, u32),
}

pub fn create_upload_info(
    uuid: String,
    name: String,
    repo: String,
    range: (u32, u32),
) -> UploadInfo {
    UploadInfo {
        uuid,
        name,
        repo,
        range,
    }
}

impl UploadInfo {

    //Move this
    pub fn uuid_exists(handler: State<bh::BackendHandler>, layer: &Layer) -> Result<bool, Error> {
        let backend = handler.backend();
        let mut req = backend::Layer::new();
        req.set_name(layer.name.to_owned());
        req.set_repo(layer.repo.to_owned());
        req.set_digest(layer.digest.to_owned());

        let response = backend.uuid_exists(&req)?;
        debug!("UuidExists: {:?}", response.get_success());
        if response.get_success() {
            Ok(true)
        } else {
            Err(errors::Error::DigestInvalid.into())
        }
    }
}

/// Gets the base URL e.g. <http://registry:8000> using the HOST value from the request header.
/// Falls back to hostname if it doesn't exist.
///
fn get_base_url(req: &Request) -> String {
    let host = match req.headers().get("HOST").next() {
        None => {
            hostname::get_hostname().expect("Server has no name; cannot give clients my address")
        }
        Some(shost) => shost.to_string(),
    };

    //TODO: Dynamically figure out whether to use http or https
    format!("https://{}", host)
}

impl<'r> Responder<'r> for UploadInfo {
    fn respond_to(self, req: &Request) -> Result<Response<'r>, Status> {
        match self {
            UploadInfo {
                ref uuid,
                ref name,
                ref repo,
                ref range,
            } => {
                debug!("Uuid Ok");
                let location_url = format!(
                    "{}/v2/{}/{}/blobs/uploads/{}",
                    get_base_url(req),
                    name,
                    repo,
                    uuid
                );
                let &(left, right) = range;
                let upload_uuid = Header::new("Docker-Upload-UUID", uuid.clone());
                let range = Header::new("Range", format!("{}-{}", left, right));
                let length = Header::new("X-Content-Length", format!("{}", right - left));
                let location = Header::new("Location", location_url);

                debug!("Range: {}-{}, Length: {}", left, right, right - left);
                Response::build()
                .header(upload_uuid)
                .header(location)
                .header(range)
                .header(length)
                // TODO: move into the type so it is better encoded?...
                .status(Status::Accepted)
                .ok()
            }
        }
    }
}

#[cfg(test)]
mod test {
    use response::upload_info::UploadInfo;
    use rocket::http::Status;

    use response::test_helper::test_route;
    fn build_response() -> UploadInfo {
        UploadInfo {
            // TODO: keep this as a real Uuid!
            uuid: String::from("whatever"),
            name: String::from("moredhel"),
            repo: String::from("test"),
            range: (0, 0),
        }
    }

    #[test]
    fn uuid_uuid() {
        let response = test_route(build_response());
        let headers = response.headers();
        assert_eq!(response.status(), Status::Accepted);
        assert!(headers.contains("Docker-Upload-UUID"));
        assert!(headers.contains("Location"));
        assert!(headers.contains("Range"));
    }

}
