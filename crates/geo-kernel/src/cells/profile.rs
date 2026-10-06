// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Content identities of the mathematical assignment and ellipsoid profile.

use purrdf_hash::Domain;

use crate::profile::PreparedEllipsoid;
use purrdf_hash::blake3::Hasher;
use purrdf_hash::frame::frame_le_into;
use purrdf_hash::hex::Digest32;
use std::sync::OnceLock;

const GRID_PROFILE: Domain = Domain::new(b"purrdf-geo-kernel/grid-profile/v1");
static WGS84_ID: OnceLock<GridProfileId> = OnceLock::new();
static CGCS2000_ID: OnceLock<GridProfileId> = OnceLock::new();

/// Normative discrete operations only: implementation and policy do not enter it.
const ASSIGNMENT_LAW: &[u8] = b"geodetic-normal;faces=+X,+Y,+Z,-X,-Y,-Z;\
charts=(1,u,v),(-u,1,v),(-u,-v,1),(-1,-v,-u),(v,-1,-u),(v,u,-1);\
warp=sign(w)*(2*abs(w)+w*w)/3,w=2*s-1;half-open-last-closed;\
quarter=RN-even(degrees/90),remainder=degrees-90*quarter;\
degrees96=RN-even(abs(remainder)*2^96);pi192=RN-even(pi*2^192);\
pi96=RN-even(pi192/2^96);angle96=RN-even(degrees96*pi96/(180*2^96));\
sin=x*Horner((-1)^k/(2k+1)!,k=0..9),cos=Horner((-1)^k/(2k)!,k=0..9);\
coefficients96=RN-even;products96=RN-even;trig62=RN-even;products62=RN-even;\
poles=(0,0,+-2^62);dominant-ties=face-order;\
bins=exact-cross-products-at-level30;\
Hilbert=T0(y/2,x/2),T1(x/2,(y+1)/2),T2((x+1)/2,(y+1)/2),T3(1-y/2,(1-x)/2);\
odd-face-root-transpose;ancestors-from-leaf;\
key=face3,path2-per-level,sentinel1,60-2*level-zeroes;levels=0..30";

/// Opaque content identity of the native grid law and geographic ellipsoid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GridProfileId(Digest32);

impl GridProfileId {
    /// Wrap a stored profile digest. This does not admit a new executable grid.
    #[must_use]
    pub const fn from_digest(digest: Digest32) -> Self {
        Self(digest)
    }

    /// The fixed-width content digest carried by an external-store bucket.
    #[must_use]
    pub const fn digest(self) -> Digest32 {
        self.0
    }

    /// The native WGS84 grid profile.
    #[must_use]
    pub fn wgs84() -> Self {
        NativeGridProfile::Wgs84.id()
    }

    /// The native CGCS2000 grid profile.
    #[must_use]
    pub fn cgcs2000() -> Self {
        NativeGridProfile::Cgcs2000.id()
    }
}

/// Geographic ellipsoids supported by the native assignment law.
///
/// Selection is explicit and carries no cross-datum operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NativeGridProfile {
    /// Semimajor axis 6378137 metres; inverse flattening 298.257223563.
    Wgs84,
    /// Semimajor axis 6378137 metres; inverse flattening 298.257222101.
    Cgcs2000,
}

impl NativeGridProfile {
    /// Hash the frozen assignment law and normalized rational ellipsoid values.
    ///
    /// Each field is framed with an eight-byte little-endian length. Parameter
    /// fields are semimajor numerator/denominator, then inverse-flattening
    /// numerator/denominator. Neither profile names nor resource limits occur.
    #[must_use]
    pub fn id(self) -> GridProfileId {
        let cached = match self {
            Self::Wgs84 => &WGS84_ID,
            Self::Cgcs2000 => &CGCS2000_ID,
        };
        *cached.get_or_init(|| self.compute_id())
    }

    fn compute_id(self) -> GridProfileId {
        let ellipsoid = self.ellipsoid();
        let mut hasher = Hasher::new();
        for field in [GRID_PROFILE.as_bytes(), ASSIGNMENT_LAW] {
            frame_le_into(&mut hasher, field);
        }
        for field in [
            ellipsoid.semimajor().numerator(),
            ellipsoid.semimajor().denominator(),
            ellipsoid.inverse_flattening().numerator(),
            ellipsoid.inverse_flattening().denominator(),
        ] {
            frame_le_into(&mut hasher, field.to_string().as_bytes());
        }
        GridProfileId(Digest32::new(*hasher.finalize().as_bytes()))
    }

    /// Prepare the native axes from their single geographic parameter home.
    #[must_use]
    pub fn ellipsoid(self) -> PreparedEllipsoid {
        match self {
            Self::Wgs84 => PreparedEllipsoid::wgs84(),
            Self::Cgcs2000 => PreparedEllipsoid::cgcs2000(),
        }
    }
}
