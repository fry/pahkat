use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::error::Error;
use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use cursed::{FromForeign, InputType, ReturnType, ToForeign};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::package_store::PackageStore;
use crate::repo::RepoRecord;
use crate::transaction::{
    PackageAction, PackageStatus, PackageStatusError, PackageTransactionError,
};
use crate::{PackageKey, PrefixPackageStore, StoreConfig};

use super::{JsonMarshaler, PackageKeyMarshaler};

pub type PrefixTarget = ();
pub type PrefixPackageAction = crate::transaction::PackageAction<PrefixTarget>;
pub type PrefixPackageTransaction = crate::transaction::PackageTransaction<PrefixTarget>;

#[cthulhu::invoke(return_marshaler = "cursed::ArcMarshaler::<PrefixPackageStore>")]
pub extern "C" fn pahkat_prefix_package_store_open(
    #[marshal(cursed::PathMarshaler)] prefix_path: PathBuf,
) -> Result<Arc<PrefixPackageStore>, Box<dyn Error>> {
    PrefixPackageStore::open(prefix_path).map(|x| Arc::new(x))
}

#[cthulhu::invoke(return_marshaler = "cursed::ArcMarshaler::<PrefixPackageStore>")]
pub extern "C" fn pahkat_prefix_package_store_create(
    #[marshal(cursed::PathMarshaler)] prefix_path: PathBuf,
) -> Result<Arc<PrefixPackageStore>, Box<dyn Error>> {
    PrefixPackageStore::create(prefix_path).map(|x| Arc::new(x))
}

#[cthulhu::invoke]
pub extern "C" fn pahkat_prefix_package_store_status(
    #[marshal(cursed::ArcRefMarshaler::<PrefixPackageStore>)] handle: Arc<PrefixPackageStore>,
    #[marshal(PackageKeyMarshaler)] package_key: PackageKey,
) -> i8 {
    super::status_to_i8(handle.status(&package_key, &()))
}

#[cthulhu::invoke(return_marshaler = "JsonMarshaler")]
pub extern "C" fn pahkat_prefix_package_store_all_statuses(
    #[marshal(cursed::ArcRefMarshaler::<PrefixPackageStore>)] handle: Arc<PrefixPackageStore>,
    #[marshal(JsonMarshaler)] repo_record: RepoRecord,
) -> BTreeMap<String, i8> {
    let statuses = handle.all_statuses(&repo_record, &());
    statuses
        .into_iter()
        .map(|(id, result)| (id, super::status_to_i8(result)))
        .collect()
}

#[cthulhu::invoke(return_marshaler = "cursed::PathMarshaler")]
pub extern "C" fn pahkat_prefix_package_store_import(
    #[marshal(cursed::ArcRefMarshaler::<PrefixPackageStore>)] handle: Arc<PrefixPackageStore>,
    #[marshal(PackageKeyMarshaler)] package_key: PackageKey,
    #[marshal(cursed::PathMarshaler)] installer_path: PathBuf,
) -> Result<PathBuf, Box<dyn Error>> {
    handle.import(&package_key, &installer_path)
}

#[cthulhu::invoke(return_marshaler = "cursed::PathMarshaler")]
pub extern "C" fn pahkat_prefix_package_store_download(
    #[marshal(cursed::ArcRefMarshaler::<PrefixPackageStore>)] handle: Arc<PrefixPackageStore>,
    #[marshal(PackageKeyMarshaler)] package_key: PackageKey,
    progress: extern "C" fn(*const PackageKey, u64, u64),
) -> Result<PathBuf, Box<dyn Error>> {
    let package_key1 = package_key.to_owned();
    handle
        .download(
            &package_key,
            Box::new(move |cur, max| {
                progress(&package_key1 as *const _, cur, max);
            }),
        )
        .map_err(|e| Box::new(e) as _)
}

#[cthulhu::invoke(return_marshaler = "JsonMarshaler")]
pub extern "C" fn pahkat_prefix_package_store_resolve_package(
    #[marshal(cursed::ArcRefMarshaler::<PrefixPackageStore>)] handle: Arc<PrefixPackageStore>,
    #[marshal(PackageKeyMarshaler)] package_key: PackageKey,
) -> Option<pahkat_types::Package> {
    handle.resolve_package(&package_key)
}

#[cthulhu::invoke]
pub extern "C" fn pahkat_prefix_package_store_clear_cache(
    #[marshal(cursed::ArcRefMarshaler::<PrefixPackageStore>)] handle: Arc<PrefixPackageStore>,
) {
    handle.clear_cache();
}

#[cthulhu::invoke]
pub extern "C" fn pahkat_prefix_package_store_refresh_repos(
    #[marshal(cursed::ArcRefMarshaler::<PrefixPackageStore>)] handle: Arc<PrefixPackageStore>,
) {
    handle.refresh_repos();
}

#[cthulhu::invoke]
pub extern "C" fn pahkat_prefix_package_store_force_refresh_repos(
    #[marshal(cursed::ArcRefMarshaler::<PrefixPackageStore>)] handle: Arc<PrefixPackageStore>,
) {
    handle.force_refresh_repos();
}

#[cthulhu::invoke(return_marshaler = "cursed::StringMarshaler")]
pub extern "C" fn pahkat_prefix_package_store_repo_indexes(
    #[marshal(cursed::ArcRefMarshaler::<PrefixPackageStore>)] handle: Arc<PrefixPackageStore>,
) -> Result<String, Box<dyn Error>> {
    let rwlock = handle.repos();
    let guard = rwlock.read().unwrap();
    let indexes = guard.values().collect::<Vec<&_>>();
    serde_json::to_string(&indexes).map_err(|e| Box::new(e) as _)
}

#[cthulhu::invoke(return_marshaler = "cursed::ArcMarshaler::<RwLock<StoreConfig>>")]
pub extern "C" fn pahkat_prefix_package_store_config(
    #[marshal(cursed::ArcRefMarshaler::<PrefixPackageStore>)] handle: Arc<PrefixPackageStore>,
) -> Arc<RwLock<StoreConfig>> {
    handle.config()
}

#[cthulhu::invoke(return_marshaler = "cursed::BoxMarshaler::<PrefixPackageTransaction>")]
pub extern "C" fn pahkat_prefix_transaction_new(
    #[marshal(cursed::ArcRefMarshaler::<PrefixPackageStore>)] handle: Arc<PrefixPackageStore>,
    #[marshal(cursed::StrMarshaler)] actions: &str,
) -> Result<Box<PrefixPackageTransaction>, Box<dyn Error>> {
    eprintln!("{:?}", &actions);
    let actions: Vec<PrefixPackageAction> = serde_json::from_str(actions)?;
    PrefixPackageTransaction::new(handle as _, actions.clone())
        .map(|x| Box::new(x))
        .map_err(|e| e.into())
}

#[cthulhu::invoke(return_marshaler = "JsonMarshaler")]
pub extern "C" fn pahkat_prefix_transaction_actions(
    #[marshal(cursed::BoxRefMarshaler::<PrefixPackageTransaction>)] handle: &PrefixPackageTransaction,
) -> Vec<PrefixPackageAction> {
    handle.actions().to_vec()
}

#[cthulhu::invoke]
pub extern "C" fn pahkat_prefix_transaction_process(
    #[marshal(cursed::BoxRefMarshaler::<PrefixPackageTransaction>)] handle: &PrefixPackageTransaction,
    tag: u32,
    progress_callback: extern "C" fn(u32, *const libc::c_char, u32),
) {
    handle.process(move |key, event| {
        let k = PackageKeyMarshaler::to_foreign(&key).unwrap();
        progress_callback(tag, k, event.to_u32());
        // PackageKeyMarshaler::drop_foreign(k);
    })
}
