use crate::aux_var::AuxVarType;
use crate::AuxVar;
use core::fmt::{Debug, Formatter};

/// Serialized form of an auxiliary vector entry / an AT variable. Each entry is a
/// `(usize, usize)`-pair in memory.
#[repr(C, packed)]
pub(crate) struct AuxVarSerialized {
    key: AuxVarType,
    val: usize,
}

impl AuxVarSerialized {
    /// Returns the key.
    pub const fn key(&self) -> AuxVarType {
        self.key
    }

    /// Returns the raw value.
    pub const fn val(&self) -> usize {
        self.val
    }

    // fn

    // todo is das unsafe hier notwendig?!
    pub unsafe fn to_aux_var<'a>(&self) -> AuxVar<'a> {
        match self.key {
            AuxVarType::AtNull => AuxVar::new_at_null(),
            AuxVarType::AtIgnore => AuxVar::new_at_ignore(self.val()),
            AuxVarType::AtExecFd => AuxVar::new_at_exec_fd(self.val()),
            AuxVarType::AtPhdr => AuxVar::new_at_phdr(self.val()),
            AuxVarType::AtPhent => AuxVar::new_at_phent(self.val()),
            AuxVarType::AtPhnum => AuxVar::new_at_phnum(self.val()),
            AuxVarType::AtPagesz => AuxVar::new_at_pagesz(self.val()),
            AuxVarType::AtBase => AuxVar::new_at_base(self.val()),
            AuxVarType::AtFlags => AuxVar::new_at_flags(self.val()),
            AuxVarType::AtEntry => AuxVar::new_at_entry(self.val()),
            AuxVarType::AtNotelf => AuxVar::new_at_notelf(self.val()),
            AuxVarType::AtUid => AuxVar::new_at_uid(self.val()),
            AuxVarType::AtEuid => AuxVar::new_at_euid(self.val()),
            AuxVarType::AtGid => AuxVar::new_at_gid(self.val()),
            AuxVarType::AtEgid => AuxVar::new_at_egid(self.val()),
            AuxVarType::AtPlatform => todo!(),
            AuxVarType::AtHwcap => todo!(),
            AuxVarType::AtClktck => todo!(),
            AuxVarType::AtSecure => todo!(),
            AuxVarType::AtBasePlatform => todo!(),
            AuxVarType::AtRandom => todo!(),
            AuxVarType::AtHwcap2 => todo!(),
            AuxVarType::AtExecFn => todo!(),
            AuxVarType::AtSysinfo => todo!(),
            AuxVarType::AtSysinfoEhdr => todo!(),
            AuxVarType::AtL1iCachesize => todo!(),
            AuxVarType::AtL1iCachegeometry => todo!(),
            AuxVarType::AtL1dCachesize => todo!(),
            AuxVarType::AtL1dCachegeometry => todo!(),
            AuxVarType::AtL2Cachesize => todo!(),
            AuxVarType::AtL2Cachegeometry => todo!(),
            AuxVarType::AtL3Cachesize => todo!(),
            AuxVarType::AtL3Cachegeometry => todo!(),
        }
    }
}

impl Debug for AuxVarSerialized {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        if self.key.value_in_data_area() {
            write!(f, "{:?}: @ {:?}", self.key(), self.val() as *const u8)
        } else {
            write!(f, "{:?}: {:?}", self.key(), self.val() as *const u8)
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use core::mem::size_of;

    #[test]
    fn test_serialized_aux_entry_size() {
        #[cfg(target_arch = "x86")]
        assert_eq!(size_of::<AuxVarSerialized>(), 8);
        #[cfg(target_arch = "x86_64")]
        assert_eq!(size_of::<AuxVarSerialized>(), 16);
    }
}
