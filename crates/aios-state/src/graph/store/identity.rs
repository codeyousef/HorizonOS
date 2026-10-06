//! Durable Btrfs identity is independent of mount-order device numbers.
//! Other filesystems remain explicitly boot-bound, never silently durable.
use super::*;

#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
enum Filesystem {
    Btrfs { uuid:[u8;16], subvolume_id:u64, subvolume_uuid:[u8;16] },
    BootDevice { boot_id:String, device:u64 },
}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Identity { filesystem:Filesystem, inode:u64 }

impl Identity {
    pub(super) fn observe(file:&File)->Result<Self> {
        let meta=file.metadata().map_err(|_|Error::Storage)?;
        // Linux x86_64 UAPI: btrfs_ioctl_fs_info_args[1024] and
        // btrfs_ioctl_get_subvol_info_args[504]. These ioctls are read-only.
        let mut fs_info=[0u8;1024];
        let filesystem=if unsafe{libc::ioctl(file.as_raw_fd(),0x8400941f as libc::c_ulong,fs_info.as_mut_ptr())}>=0 {
            let mut subvolume=[0u8;504];
            let subvolume_result=unsafe{libc::ioctl(file.as_raw_fd(),0x81f8943c as libc::c_ulong,subvolume.as_mut_ptr())};
            // A nonnegative ioctl result is success; this kernel returns 1.
            if subvolume_result<0 {return Err(Error::Storage);}
            let uuid:[u8;16]=fs_info[16..32].try_into().map_err(|_|Error::Storage)?;
            let subvolume_id=u64::from_ne_bytes(subvolume[..8].try_into().map_err(|_|Error::Storage)?);
            let subvolume_uuid:[u8;16]=subvolume[296..312].try_into().map_err(|_|Error::Storage)?;
            // Root tree 5 has no subvolume UUID. All other trees must have one.
            if uuid==[0;16] || subvolume_id<5 || (subvolume_id!=5 && subvolume_uuid==[0;16]) {return Err(Error::IdentityChanged);}
            Filesystem::Btrfs{uuid,subvolume_id,subvolume_uuid}
        } else {
            let error=std::io::Error::last_os_error();
            if !matches!(error.raw_os_error(),Some(libc::ENOTTY|libc::EOPNOTSUPP)) {return Err(Error::Storage);}
            let boot_id=fs::read_to_string("/proc/sys/kernel/random/boot_id").map_err(|_|Error::Storage)?.trim().to_owned();
            if boot_id.len()!=36 || !boot_id.bytes().enumerate().all(|(i,c)|if [8,13,18,23].contains(&i){c==b'-'}else{c.is_ascii_hexdigit()}) {return Err(Error::Storage);}
            Filesystem::BootDevice{boot_id,device:meta.dev()}
        };
        let after=file.metadata().map_err(|_|Error::Storage)?;
        if (meta.dev(),meta.ino())!=(after.dev(),after.ino()){return Err(Error::IdentityChanged);}
        Ok(Self{filesystem,inode:meta.ino()})
    }
    pub(super) fn directory(path:&Path)->Result<Self> {
        private_directory(path,unsafe{libc::geteuid()})?;
        let before=fs::symlink_metadata(path).map_err(|_|Error::Storage)?;
        let file=OpenOptions::new().read(true).custom_flags(libc::O_DIRECTORY|libc::O_NOFOLLOW|libc::O_CLOEXEC).open(path).map_err(|_|Error::Storage)?;
        let meta=file.metadata().map_err(|_|Error::Storage)?;
        if (before.dev(),before.ino())!=(meta.dev(),meta.ino()){return Err(Error::IdentityChanged);}
        let identity=Self::observe(&file)?;
        let after=fs::symlink_metadata(path).map_err(|_|Error::Storage)?;
        if (after.dev(),after.ino())!=(meta.dev(),meta.ino()){return Err(Error::IdentityChanged);}
        Ok(identity)
    }
}
