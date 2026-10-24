//! Chunked uploads in the style of the Dropzone.js client library, used by
//! EasyUpload, AnonTransfer and Fastupload.
//!
//! Every chunk is its own multipart POST carrying `dz*` bookkeeping fields; the
//! server stitches the chunks together using the shared `dzuuid`.

use crate::error::UploadError;
use crate::reply::{Reply, SendReply};
use crate::upload::{UploadContext, chunk_ranges, new_form};

pub(crate) type Headers = Vec<(&'static str, String)>;

pub(crate) struct Dropzone<'a> {
    pub url: &'a str,
    /// Identifies this upload across chunks (`dzuuid`).
    pub uuid: &'a str,
    pub chunk_size: u64,
    /// Name of the multipart field that carries the chunk.
    pub file_field: &'static str,
    /// Extra form fields sent with every chunk.
    pub fields: Vec<(&'static str, String)>,
}

impl Dropzone<'_> {
    /// Upload the file chunk by chunk and return the reply to the last chunk,
    /// which is where servers put the download link.
    ///
    /// `after_chunk` sees each reply and may adjust `headers` for the next
    /// request (Fastupload rotates its session cookie this way).
    pub(crate) async fn upload(
        &self,
        cx: &UploadContext<'_>,
        headers: &mut Headers,
        mut after_chunk: impl FnMut(&Reply, &mut Headers),
    ) -> Result<Reply, UploadError> {
        let size = cx.file.size;
        let total_chunks = size.div_ceil(self.chunk_size);

        let mut last = None;
        for (index, (offset, len)) in chunk_ranges(size, self.chunk_size).enumerate() {
            cx.progress
                .set_message(format!("Chunk {}/{}", index + 1, total_chunks));

            let mut form = new_form()
                .text("dzuuid", self.uuid.to_string())
                .text("dzchunkindex", index.to_string())
                .text("dztotalfilesize", size.to_string())
                .text("dzchunksize", self.chunk_size.to_string())
                .text("dztotalchunkcount", total_chunks.to_string())
                .text("dzchunkbyteoffset", offset.to_string());
            for (name, value) in &self.fields {
                form = form.text(*name, value.clone());
            }
            let form = form.part(self.file_field, cx.file_part(offset, len).await?);

            let mut request = cx.http.post(self.url);
            for (name, value) in headers.iter() {
                request = request.header(*name, value);
            }
            let reply = request.multipart(form).send_reply().await?.ok()?;
            after_chunk(&reply, headers);
            last = Some(reply);
        }
        last.ok_or(UploadError::EmptyFile)
    }
}
