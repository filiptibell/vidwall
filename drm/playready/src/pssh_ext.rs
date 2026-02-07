use drm_core::{PsshBox, SystemId};
use drm_playready_format::wrm_header::{PlayReadyHeader, WrmHeader};

use crate::error::{CdmError, CdmResult};

/**
    PlayReady-specific extensions for [`PsshBox`].
*/
pub trait PlayReadyExt {
    /**
        Parse the PSSH data payload as a PlayReady Header (binary PRH container).
    */
    fn playready_header(&self) -> CdmResult<PlayReadyHeader>;

    /**
        Extract the WRM Header XML string from the first type-1 record.
    */
    fn playready_wrm_header_xml(&self) -> CdmResult<String>;

    /**
        Parse the WRM Header XML into a structured [`WrmHeader`].
    */
    fn playready_wrm_header(&self) -> CdmResult<WrmHeader>;

    /**
        Extract key IDs from the WRM Header.

        Returns KIDs in standard UUID byte order (already swapped from
        PlayReady's GUID little-endian format by the format crate).
    */
    fn playready_key_ids(&self) -> CdmResult<Vec<[u8; 16]>>;

    /**
        Check that this PSSH box uses the PlayReady system ID.
    */
    fn ensure_playready(&self) -> CdmResult<()>;
}

impl PlayReadyExt for PsshBox {
    fn playready_header(&self) -> CdmResult<PlayReadyHeader> {
        self.ensure_playready()?;
        PlayReadyHeader::from_bytes(&self.data).map_err(CdmError::from)
    }

    fn playready_wrm_header_xml(&self) -> CdmResult<String> {
        let header = self.playready_header()?;
        header
            .wrm_header_xml()
            .ok_or_else(|| CdmError::Format("no WRM Header record in PlayReady Header".into()))?
            .map_err(CdmError::from)
    }

    fn playready_wrm_header(&self) -> CdmResult<WrmHeader> {
        let xml = self.playready_wrm_header_xml()?;
        WrmHeader::from_xml(&xml).map_err(CdmError::from)
    }

    fn playready_key_ids(&self) -> CdmResult<Vec<[u8; 16]>> {
        let wrm = self.playready_wrm_header()?;
        Ok(wrm.kids.iter().map(|sk| sk.key_id).collect())
    }

    fn ensure_playready(&self) -> CdmResult<()> {
        self.ensure_system_id(SystemId::PlayReady)
            .map_err(CdmError::PsshCore)
    }
}
