use crate::prepare::{self, Capsule};
use anyhow::{Context, Result, ensure};
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};
const TRAILER: &[u8; 8] = b"NIOEXE02";
fn payload(file: &mut fs::File) -> Result<Option<(u64, Vec<u8>)>> {
    let total = file.metadata()?.len();
    if total < 16 {
        return Ok(None);
    }
    file.seek(SeekFrom::End(-16))?;
    let mut trailer = [0u8; 16];
    file.read_exact(&mut trailer)?;
    if &trailer[8..] != TRAILER {
        return Ok(None);
    }
    let size = u64::from_le_bytes(trailer[..8].try_into().unwrap());
    ensure!(
        size <= prepare::MAX_CAPSULE as u64 && size <= total - 16,
        "invalid embedded capsule length"
    );
    let start = total - 16 - size;
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = vec![0; size as usize];
    file.read_exact(&mut bytes)?;
    Ok(Some((start, bytes)))
}
pub fn embedded() -> Result<Option<Capsule>> {
    let mut file = fs::File::open(std::env::current_exe()?)?;
    payload(&mut file)?
        .map(|(_, bytes)| prepare::decode_capsule(&bytes))
        .transpose()
}
pub fn build(output: &Path, capsule: &Capsule) -> Result<()> {
    let executable = std::env::current_exe()?;
    ensure!(
        !output.exists() || fs::canonicalize(output)? != executable,
        "cannot overwrite the running runtime"
    );
    let bytes = prepare::capsule_bytes(capsule)?;
    let mut source = fs::File::open(&executable)?;
    let end = payload(&mut source)?
        .map(|(start, _)| start)
        .unwrap_or(source.metadata()?.len());
    source.seek(SeekFrom::Start(0))?;
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let mut destination = tempfile::NamedTempFile::new_in(parent)?;
    std::io::copy(&mut source.take(end), destination.as_file_mut())?;
    destination.write_all(&bytes)?;
    destination.write_all(&(bytes.len() as u64).to_le_bytes())?;
    destination.write_all(TRAILER)?;
    destination
        .as_file()
        .set_permissions(fs::metadata(executable)?.permissions())?;
    destination.as_file().sync_all()?;
    destination
        .persist(output)
        .with_context(|| format!("write standalone executable {}", output.display()))?;
    Ok(())
}
