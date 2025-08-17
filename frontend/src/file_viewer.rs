// use dioxus::prelude::*;
// use gloo_file::Blob; // Add this import at the top
//
// #[component]
// fn FileViewer(file_url: Signal<Option<String>>) -> Element {
//     rsx! {
//         div {
//             class: "file-viewer-panel",
//             if let Some(url) = file_url() {
//                 // An iframe is the standard way to embed a PDF in a web page
//                 iframe {
//                     src: "{url}",
//                     class: "pdf-iframe"
//                 }
//             } else {
//                 // Show a placeholder before a file is uploaded
//                 div {
//                     class: "file-viewer-placeholder",
//                     "Upload a document to display it here."
//                 }
//             }
//         }
//     }
// }
//
//
// #[component]
// fn FileUploader(file_url: Signal<Option<String>>) -> Element {
//     // This future will handle the asynchronous file reading
//     let mut file_upload_future = use_future(move || async move {
//         if let Some(file) = dioxus_desktop::use_window().await.file_handler().select_file(Some(&["pdf"])) {
//             let bytes = file.read().await.ok()?;
//             // Create a "Blob" from the raw file bytes
//             let blob = Blob::new_with_options(bytes.as_slice(), Some("application/pdf"));
//             // Create a temporary URL that the iframe can use to access the blob
//             let url = web_sys::Url::create_object_url_with_blob(&blob).ok()?;
//
//             // Set the signal to update the FileViewer
//             file_url.set(Some(url));
//         }
//         Some(())
//     });
//
//     rsx! {
//         button {
//             class: "control-button",
//             // When clicked, poll the future to open the file dialog
//             onclick: move |_| {
//                 file_upload_future.run();
//             },
//             "Upload Document (PDF)"
//         }
//     }
// }
