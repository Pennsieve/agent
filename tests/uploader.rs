use actix;

#[macro_use]
extern crate lazy_static;
use time;

use actix::prelude::*;

use futures::*;

use std::collections::HashMap;

use pennsieve_rust::model::upload::MultipartUploadId;
use pennsieve_rust::model::{DatasetNodeId, ImportId, OrganizationId, UploadId};

use pennsieve::api::Api;
use pennsieve::config::Config;
use pennsieve::database::{UploadRecord, UploadStatus};
use pennsieve::{upload, util, WithProps};

lazy_static! {
    // "Agent Testing"
    static ref FIXTURE_ORGANIZATION_NODE_ID: String = String::from("N:organization:713eeb6e-c42c-445d-8a60-818c741ea87a");
    static ref FIXTURE_ORGANIZATION_INT_ID: u32 = 5;
    static ref FIXTURE_ORGANIZATION: OrganizationId = OrganizationId::new((*FIXTURE_ORGANIZATION_NODE_ID).clone());
}

fn file_upload_path(package_name: &str) -> String {
    format!("test_resources/upload_test/{}", package_name)
}

fn in_progress_retry_record(
    id: &DatasetNodeId,
    package_name: &String,
    package_to_metadata: &HashMap<String, (ImportId, MultipartUploadId)>,
) -> UploadRecord {
    let (import_id, multipart_upload_id) = package_to_metadata.get(package_name).unwrap().clone();
    UploadRecord {
        id: None,
        file_path: file_upload_path(package_name.as_str()),
        dataset_id: id.clone().into(),
        package_id: None,
        import_id: import_id.to_string(),
        progress: 50,
        status: UploadStatus::InProgress,
        // 10 minutes old:
        created_at: time::now().to_timespec() - time::Duration::minutes(10),
        updated_at: time::now().to_timespec() - time::Duration::minutes(10),
        append: false,
        upload_service: true,
        organization_id: (*FIXTURE_ORGANIZATION_NODE_ID).clone(),
        chunk_size: Some(100),
        multipart_upload_id: Some(multipart_upload_id.0),
    }
}

fn in_progress_aged_record(
    id: &DatasetNodeId,
    package_name: &String,
    package_to_metadata: &HashMap<String, (ImportId, MultipartUploadId)>,
) -> UploadRecord {
    let (import_id, multipart_upload_id) = package_to_metadata.get(package_name).unwrap().clone();
    UploadRecord {
        id: None,
        file_path: file_upload_path(package_name.as_str()),
        dataset_id: id.clone().into(),
        package_id: None,
        import_id: import_id.to_string(),
        progress: 99,
        status: UploadStatus::InProgress,
        created_at: time::now().to_timespec() - time::Duration::weeks(1), // age 1 week
        updated_at: time::now().to_timespec() - time::Duration::weeks(1),
        append: false,
        upload_service: true,
        organization_id: (*FIXTURE_ORGANIZATION_NODE_ID).clone(),
        chunk_size: Some(100),
        multipart_upload_id: Some(multipart_upload_id.0),
    }
}

fn queued_record(
    id: &DatasetNodeId,
    package_name: &String,
    package_to_metadata: &HashMap<String, (ImportId, MultipartUploadId)>,
) -> UploadRecord {
    let (import_id, multipart_upload_id) = package_to_metadata.get(package_name).unwrap().clone();
    UploadRecord {
        id: None,
        file_path: file_upload_path(package_name.as_str()),
        dataset_id: id.clone().into(),
        package_id: None,
        import_id: import_id.to_string(),
        progress: 0,
        status: UploadStatus::Queued,
        created_at: time::now().to_timespec(),
        updated_at: time::now().to_timespec(),
        append: false,
        upload_service: true,
        organization_id: (*FIXTURE_ORGANIZATION_NODE_ID).clone(),
        chunk_size: Some(100),
        multipart_upload_id: Some(multipart_upload_id.0),
    }
}

fn test_ini() -> String {
    format!(
        r#"
        [global]
        default_profile=Agent_Testing

        [Agent_Testing]
        api_token={}
        api_secret={}
        environment=nonproduction

        [agent]
        metrics = true
        cache_page_size = 10000
        cache_base_path = "~/.pennsieve/cache"
        cache_soft_cache_size = 5000000000
        cache_hard_cache_size = 10000000000
        proxy = true
        proxy_local_port = 8080
        timeseries = true
        timeseries_local_port = 9500
        uploader = true
    "#,
        env!("PENNSIEVE_API_KEY"),
        env!("PENNSIEVE_SECRET_KEY")
    )
}

#[test]
pub fn test_complex_file_uploads() {
    let db = util::database::temp().unwrap();

    let config: Config = test_ini().parse().unwrap();

    let api = Api::new(
        &db.clone(),
        &config,
        config
            .api_settings
            .get_profile("Agent_Testing")
            .unwrap()
            .environment,
    );

    println!("Start test_complex_file_uploads");

    let props = upload::Props {
        api: api.clone(),
        db: db.clone(),
        parallelism: num_cpus::get(),
    };
    upload::Uploader::with_props(props);

    let worker = upload::Uploader;

    let create_dataset = api
        .create_dataset(
            util::strings::random_suffix("agent-upload-worker-test"),
            None as Option<String>,
        )
        .map_err(|e| {
            println!("CREATE DATASET: {:?}", e);
            e
        });

    let file_paths = vec![
        String::from("test_resources/upload_test/1.txt"),
        String::from("test_resources/upload_test/2.txt"),
        String::from("test_resources/upload_test/3.txt"),
        #[cfg(not(windows))]
        String::from("test_resources/upload_test/.ignore.txt"),
    ];

    let fut = future::ok(db.clone())
        .join4(future::ok(api), future::ok(file_paths), create_dataset)
        // Canonicalize file paths (will strip out hidden files):
        .and_then(move |(db, api, file_paths, dataset)| {
            upload::generate_file_preview(file_paths, true)
                .map(|preview| (db, api, dataset, preview))
                .map_err(Into::into)
        })
        .and_then(move |(db, api, dataset, agent_preview)| {
            let ps = api.client();
            let ds = dataset.take();
            let id = ds.id().clone();
            let int_id = ds.int_id().clone();

            ps.preview_upload(
                &(*FIXTURE_ORGANIZATION),
                &int_id,
                agent_preview.path(),
                agent_preview.file_paths(),
                false,
                false,
            )
            .map(|preview| (db, api, id, preview))
            .map_err(Into::into)
        })
        .and_then(|(db, api, id, preview)| {
            let package_to_metadata: HashMap<String, (ImportId, MultipartUploadId)> = preview
                .packages()
                .iter()
                .cloned()
                .map(|p| {
                    (
                        p.package_name().to_string(),
                        (
                            p.import_id().clone(),
                            p.files()
                                .first()
                                .unwrap()
                                .clone()
                                .multipart_upload_id()
                                .unwrap()
                                .clone(),
                        ),
                    )
                })
                .collect();

            let records = vec![
                in_progress_retry_record(&id, &String::from("1.txt"), &package_to_metadata),
                queued_record(&id, &String::from("2.txt"), &package_to_metadata),
                in_progress_aged_record(&id, &String::from("3.txt"), &package_to_metadata),
            ];

            for ref mut record in &records {
                db.insert_upload(record).unwrap();
            }

            Ok((db, api, id, package_to_metadata))
        })
        .and_then(move |(db, api, id, package_to_metadata)| {
            // Run one step of the upload worker:
            worker
                .step()
                .map(move |_| (db, api, id, package_to_metadata))
        })
        .and_then(move |(db, api, id, package_to_metadata)| {
            api.delete_dataset(id).map(|_| (db, package_to_metadata))
        })
        .and_then(move |(db, package_to_metadata)| {
            System::current().stop();

            let (import_id_1, _) = package_to_metadata.get(&String::from("1.txt")).unwrap();
            let (import_id_2, _) = package_to_metadata.get(&String::from("2.txt")).unwrap();
            let (import_id_3, _) = package_to_metadata.get(&String::from("3.txt")).unwrap();
            // Hidden files should not be in `package_to_metadata`:
            #[cfg(not(windows))]
            {
                assert_eq!(
                    package_to_metadata.contains_key(&String::from(".ignore")),
                    false
                );
            }
            // Check final state of uploads:

            // Retry -> back to in progress:
            assert_eq!(
                db.get_uploads_by_import_id(&import_id_1.to_string())
                    .unwrap()
                    .into_owned_iter()
                    .map(|r| (r.status, r.progress))
                    .collect::<Vec<(UploadStatus, i32)>>(),
                vec![(UploadStatus::InProgress, 50)]
            );

            // Queued uploads should move to completed:
            assert_eq!(
                db.get_uploads_by_import_id(&import_id_2.to_string())
                    .unwrap()
                    .into_owned_iter()
                    .map(|r| r.status)
                    .collect::<Vec<UploadStatus>>(),
                vec![UploadStatus::Completed]
            );

            // Expired / aged out becomes failed:
            assert_eq!(
                db.get_uploads_by_import_id(&import_id_3.to_string())
                    .unwrap()
                    .into_owned_iter()
                    .map(|r| r.status)
                    .collect::<Vec<UploadStatus>>(),
                vec![UploadStatus::Failed]
            );

            Ok(())
        });

    let system = System::new("ps:main");
    Arbiter::spawn(fut.map(|_| ()).map_err(|_| ()));
    system.run();

    println!("uploader: checks passed");
}
