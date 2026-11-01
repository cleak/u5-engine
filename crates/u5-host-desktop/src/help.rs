//! Player-facing help text for desktop setup (our own wording).

/// "How to get the files", for the desktop wizards.
pub const HOW_TO_GET_THE_FILES: &str = "\
This engine is a new program that plays Ultima V: Warriors of Destiny, but it
does not include the game itself. You need the original DOS game files, such
as BRIT.DAT, TILES.16 and TOWNE.DAT.

Buying the game
  Ultima V is sold on GOG.com (on its own or in the Ultima 4+5+6 bundle). It
  is the release this engine is tested against.

Windows
  Install the game with GOG Galaxy or the offline installer. Setup looks in
  C:\\GOG Games and the GOG Galaxy library automatically; otherwise choose the
  game's folder (the one containing BRIT.DAT).

Linux and macOS
  Download GOG's Windows offline installer (setup_ultima_5_*.exe) and unpack
  it with innoextract, which most package managers provide:

      innoextract --gog --output-dir ~/GOG\\ Games/Ultima\\ 5 setup_ultima_5_*.exe

  Then choose that folder (setup searches a few subfolders deep). Copies
  installed through Heroic, Lutris, Bottles or Wine are also found.

From an old floppy or CD installation
  Copy the whole game folder from your DOS installation. Lower-case file
  names are fine; setup makes its own correctly named copy.

Your files are only read. Setup copies them into the engine's own data folder,
and your saved games are kept there.
";
