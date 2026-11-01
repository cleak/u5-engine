//! Known game-data releases, identified by file size and SHA-256.
//!
//! Only the static files the engine reads are hashed; saves change as the
//! game is played and are never part of an identification. The hashes
//! identify a release without carrying any of its content.

/// One hashed file of a known release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnownFile {
    pub name: &'static str,
    pub size: u64,
    /// Lower-case hex SHA-256.
    pub sha256: &'static str,
}

/// A release the engine has been verified against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnownVersion {
    /// Stable identifier, written to configuration files.
    pub id: &'static str,
    /// Human-readable name.
    pub name: &'static str,
    pub files: &'static [KnownFile],
}

/// The GOG.com DOS release (game version 1.0), the verified data set.
pub const GOG_1_0: KnownVersion = KnownVersion {
    id: "gog-1.0",
    name: "GOG.com release (DOS version 1.0)",
    files: GOG_1_0_FILES,
};

/// Every release this engine knows.
pub const KNOWN_VERSIONS: &[KnownVersion] = &[GOG_1_0];

pub fn known_version(id: &str) -> Option<&'static KnownVersion> {
    KNOWN_VERSIONS.iter().find(|version| version.id == id)
}

#[rustfmt::skip]
const GOG_1_0_FILES: &[KnownFile] = &[
    KnownFile { name: "BRIT.CBT", size: 5632, sha256: "ea93924f649eb2406df8507f008e6eadc7dbf54b8822ea6cda6f44fbd3429aac" },
    KnownFile { name: "BRIT.DAT", size: 52480, sha256: "21c853dd786f3f580184b91aac448adf9b204c8071c5ad1f5da277665b7379a6" },
    KnownFile { name: "BRITISH.BIT", size: 784, sha256: "a451007f4562591323cbc6251a958765a341a4d4f600b6dd27f45be2062c56d5" },
    KnownFile { name: "BRITISH.PTH", size: 2783, sha256: "badfdb61772b45b7c9543ee1df4996271a9bfd618bb159a184b60111a88ba4b3" },
    KnownFile { name: "CASTLE.DAT", size: 16384, sha256: "52de7aee874ac02c57886846e4545f5aceeaec85fb3da83b6662d115190a481a" },
    KnownFile { name: "CASTLE.NPC", size: 4608, sha256: "6327fbbc9934a020400fa8cc6ba4e56e17dc5f4734589da1d2e48c5eb34e2a1b" },
    KnownFile { name: "CASTLE.TLK", size: 21868, sha256: "539137cd980655a81e1de1361a174a4e682eceb4dcc3afb0de4716f961187eb2" },
    KnownFile { name: "CREATE.16", size: 11000, sha256: "93557759eeca6f2c79575f7dde072edf3cbc03d9005bd8778efa7059eaac32a8" },
    KnownFile { name: "DATA.OVL", size: 48464, sha256: "da7d10decb8864754d13910e8694c0cb7301b5224f25156730857827f07168c3" },
    KnownFile { name: "DNG1.16", size: 8929, sha256: "dbfd6c1580eb423c77127e904ec16f98728acd62c7750061f5a48500d8342e39" },
    KnownFile { name: "DNG2.16", size: 18552, sha256: "47a40483788fc8756bad7b234f461d5f0ca3069a93e4f7969e3511e7bc2bb20e" },
    KnownFile { name: "DNG3.16", size: 17020, sha256: "7d3f3e6de177bf2d3832d805d78cf212cb93123f9ad28a9659f48d63f42b3561" },
    KnownFile { name: "DUNGEON.CBT", size: 39424, sha256: "101f127141b5e90d7186263e47c81ee496c0e6a44acbe0a39eadc646090f9936" },
    KnownFile { name: "DUNGEON.DAT", size: 4096, sha256: "0218c9efe54ee6fd1e33678a14923feb76a5857d56c7ec1bb021c0b5eb069ce5" },
    KnownFile { name: "DWELLING.DAT", size: 16384, sha256: "fb268f2faa0b9ee6e42e12202144572ede3772b9d8a1d7b2047e1fbbc5f789c5" },
    KnownFile { name: "DWELLING.NPC", size: 4608, sha256: "cdf6361fec964abece7deb28e379276807386f2af80e46f581f0eae7b4ed906b" },
    KnownFile { name: "DWELLING.TLK", size: 7979, sha256: "c1a78ba3929996ec4057d3d0154936199aebc2700879867ebd245aefd6c9859d" },
    KnownFile { name: "END1.16", size: 7792, sha256: "74e7d49b31c17f598b55e43ea5125db52ba79be7bc88f82c4ee34fdd0d5be489" },
    KnownFile { name: "END2.16", size: 8542, sha256: "e8b395b7bca403c294d4f5544cce577cce11a93a6c05b81375b873324af8fbe0" },
    KnownFile { name: "END.DAT", size: 3698, sha256: "1ccdee2751731d6e9769adbcb2734ff967fc878691c3dad2699cc1e9fc9caeb4" },
    KnownFile { name: "ENDMSG.DAT", size: 786, sha256: "60ea23e2e4cf6e9a58987c41bc72e808d0f6ccd49553f6bfdcc4281a5a59ebdf" },
    KnownFile { name: "ENDSC.16", size: 1392, sha256: "d096c4caffab3e624db0cefa8b50b670f6e4d9db6dc4eb3d7e65c38cb9d23799" },
    KnownFile { name: "IBM.CH", size: 1024, sha256: "3f62e1868cea2387c2b503b0193157c02928cdd502fc272cb50fcd8d613c5c83" },
    KnownFile { name: "INIT.GAM", size: 4192, sha256: "6944efc74169018392e3add38a6142b9e17412c8c8eba28fc0f5185e9bc29de0" },
    KnownFile { name: "INIT.OOL", size: 256, sha256: "313e0e1830fbbe868c58123ce98865a003f02629bc544addedeae293af87c419" },
    KnownFile { name: "ITEMS.16", size: 2793, sha256: "8a1689ad0af4d26a48f6b64c5470d0f4a04063b418030932805ab185afc2a687" },
    KnownFile { name: "KARMA.DAT", size: 761, sha256: "94e92100331286e35d03e8d4f0b32bdecbc69bc3b23f3a4ec64d775f1e3bf94c" },
    KnownFile { name: "KEEP.DAT", size: 16384, sha256: "7471fbd4c9b22213fc7bd3e28478a25610e77cc0ba26f1add873e4a3d9aaf0cf" },
    KnownFile { name: "KEEP.NPC", size: 4608, sha256: "81bb174e9dcc37b7691fa6ef679f2909f83b85c50ee73417bb822fd3629dee63" },
    KnownFile { name: "KEEP.TLK", size: 17947, sha256: "29c7337739bf235cfd825f74676611261f754d37996a5c9b2bd910ad3d0d6adb" },
    KnownFile { name: "LOOK2.DAT", size: 3622, sha256: "a4b99c7d8495552068599ce7a4b0df3811cad99ff0c8ecbdfaf115ac46409f1b" },
    KnownFile { name: "MISCMAPS.DAT", size: 1871, sha256: "be170b727e86ffd0e50c87b5b04aadde490c2fd41cde982fb5be00980f5fc127" },
    KnownFile { name: "MISCMSG.DAT", size: 2745, sha256: "c882f10f5236b92285571735f17ba8807068aeef9c6abaae17a8fe77f0b85c66" },
    KnownFile { name: "MON0.16", size: 993, sha256: "c16f38f7c28e39992e148e610881b417c78856e044b1572efada6429f8accdaf" },
    KnownFile { name: "MON1.16", size: 560, sha256: "349264cf1fb886138720cef79326c668e681ca4c986548174c73a89c32ba7b4c" },
    KnownFile { name: "MON2.16", size: 842, sha256: "40875ea577c69f6f4f18fb4c8bce053819fffd615be4545c8d5adaa898b72262" },
    KnownFile { name: "MON3.16", size: 722, sha256: "44c3b2142649e69f9c0cd93d5338599db4c0a78ec3885750b351d15c92267ce9" },
    KnownFile { name: "MON4.16", size: 626, sha256: "3ce41f20cb5b31fbedb83acd4285cfda5a3d3a3203290f7ff4c06a8803f8c158" },
    KnownFile { name: "MON5.16", size: 1276, sha256: "96e7aa9b43e7d80139c71d352e63299101f3c1474fde2bdbd70d9484be48ace2" },
    KnownFile { name: "MON6.16", size: 860, sha256: "2e9de46c93cf23030211f2d68d19d0d06ca38d69f466bc184b302a8f0b856760" },
    KnownFile { name: "MON7.16", size: 1063, sha256: "1c57d173e1014756ced2f6a17a865c383b5011d002493272c4943b3808698a1d" },
    KnownFile { name: "PROPORT.PCS", size: 802, sha256: "424684721569465f65907a914d32bb6ad621cbb2c52bead36cbf74dfa0f88be5" },
    KnownFile { name: "QUESTION.DAT", size: 7746, sha256: "3fbe5d114f947aa43d877ace6de5a2b913fe6ce17ff7237f2ef9d1d255673c8f" },
    KnownFile { name: "RUNES.CH", size: 1024, sha256: "671d514b74a4419804e3c18babd7b7d000aa656f8ffb045eac5859437f14a6ff" },
    KnownFile { name: "SHOPPE.DAT", size: 10135, sha256: "fd79461a18e3c146d0ce50adf0066a047b26803eaace34f3e9f3c8bf25e16b7d" },
    KnownFile { name: "SIGNS.DAT", size: 8364, sha256: "f8d28fb90372c395f795650a30522ff30097546615bea4b9e206aa3b755081cd" },
    KnownFile { name: "STARTSC.16", size: 3383, sha256: "4bf286a729a42e0a5d62f68f1701d92ccf50ee72d2dccd7077152f828cf819d1" },
    KnownFile { name: "STORY1.16", size: 7725, sha256: "271c6390c10af5a56c708d8cd4c570a97ed375f124ad1164d3d2c236ec5795df" },
    KnownFile { name: "STORY2.16", size: 14461, sha256: "a26f0849cc21ce0bca8d87c9935f9ac635ec48928963315aae119029eaf80008" },
    KnownFile { name: "STORY3.16", size: 9274, sha256: "7aec4bc0c091344473f03a747d95404164f72fb9ae0f7155a9a39fbd19d11cfe" },
    KnownFile { name: "STORY4.16", size: 11190, sha256: "98ef0fb976924b8300b1e965ff190aceebc3d4463b9de266e8188c6ec0871b54" },
    KnownFile { name: "STORY5.16", size: 12336, sha256: "d8791fd344101e032a826e1f0ba2f6a9caf54f28dd0e7a8b3a49dd128fb058f9" },
    KnownFile { name: "STORY6.16", size: 15290, sha256: "f11ab3e6286b6c0ae8962355ad4afcbdbe00e587a6a10cb4ab69433734737d9b" },
    KnownFile { name: "STORY.DAT", size: 11679, sha256: "a0fd073c621418d718977c19fb93189c3da93dd3a0cb427f9188b2aec1c70132" },
    KnownFile { name: "TEXT.16", size: 1678, sha256: "f106c8ba099c074510215ce1e4b7886b09eaf128475e85dd56e2808c83342a33" },
    KnownFile { name: "TILES.16", size: 30414, sha256: "bc5d33bdb92a9f054cabf95fc34293d65b181e23eabf041a08edf88027ea4f83" },
    KnownFile { name: "TITLE.BIT", size: 3323, sha256: "79a133a29a1212988334e18a2eddde9e1f23377b31cfa539af3bb9f169cbb90d" },
    KnownFile { name: "TOWNE.DAT", size: 16384, sha256: "941b186e69eee961a614902eaabec16495a113bb08913a80d7630724fa7ea0dc" },
    KnownFile { name: "TOWNE.NPC", size: 4608, sha256: "6c62bed1d4fba8821184ce3b59e82bf20ee34e2e54f80690de34fea971c51d5d" },
    KnownFile { name: "TOWNE.TLK", size: 26835, sha256: "eb586f6f6e736d1023fdc949e1fe56a539437776ec358536ccd356efc7210951" },
    KnownFile { name: "ULTIMA.16", size: 16039, sha256: "a0089c126316b988bb0f15c516641b6b188994557861ad0f3ffba7ca750addcd" },
    KnownFile { name: "UNDER.DAT", size: 65536, sha256: "7627aa65ede0676b27cd6fcbd0ef434935ca679506e3743b50fc7d9d14cf2936" },
    KnownFile { name: "WD.BIT", size: 1772, sha256: "35104934ba9deb040b5378298bcfaf3d355494d94e3580d6fd99bd7d5ba10e48" },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{MANIFEST, manifest_entry};

    #[test]
    fn gog_table_covers_exactly_the_required_files() {
        let required: Vec<&str> = MANIFEST
            .iter()
            .filter(|file| file.is_required())
            .map(|file| file.name)
            .collect();
        assert_eq!(GOG_1_0.files.len(), required.len());
        for file in GOG_1_0.files {
            let entry = manifest_entry(file.name).expect("hashed file is in the manifest");
            assert!(
                entry.is_required(),
                "{} is hashed but not required",
                file.name
            );
            assert_eq!(file.sha256.len(), 64);
            assert!(
                file.sha256
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            );
        }
    }
}
