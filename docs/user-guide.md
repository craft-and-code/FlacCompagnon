# User guide

FlacCompagnon brings library analysis, listening, tag editing and conversion together. This guide follows a simple flow: drop some tracks, browse the results, then change only what you choose to change.

## Before you start

You can drop one audio file or a whole folder. Subfolders are scanned, and files already in the list are not added twice.

Analysis never changes your files. Writing to disk always requires a clear action: saving tags or artwork, renaming or renumbering a track, converting, generating spectrograms or saving a report.

## Analyze files by drag and drop

1. Open FlacCompagnon.
2. Drop a folder or audio files on the large central area.
3. Wait for the analysis to finish. The list shows the files, their properties and the useful findings.

The **Add titles…** button also opens a folder picker. Use it when you prefer a file dialog to drag and drop.

Each row represents one track. Visible columns can show the format, sample rate, bit depth, a detection and level measurements. A result calls attention to a measured property; it does not replace listening.

> **On-screen cue:** use the small magnifier on a row to reveal the file in Finder or Explorer. The nearby play button starts that track.

To rename a track, select its row, then click its name again. **Enter** applies the new name while keeping the extension; **Escape** cancels. Renaming never replaces an existing destination.

To understand a column, open its article in [Analyses](index.md). Every article explains what the measurement observes and its limitations.

## Edit tags and artwork

Select one or more tracks in the list. The **Tags** panel opens on the left.

It contains the album artwork and the common fields: title, artist, album, album artist, year, genre, track number and comment. Edit one or more fields, then use **Save** at the bottom of the panel to write the changes to the selected files.

Artwork follows the role chosen in the list beneath the cover, such as _Front cover_ or _Back cover_. Change the role to display the matching image. You can:

- drop an image on the cover to stage it for the selected tracks;
- use the export button below the image to extract it next to the audio files;
- use the trash button to stage its deletion.

Local and embedded artwork is limited to 12 MiB and 64 megapixels. Resize an oversized image before adding it.

Image headers must provide valid dimensions. Classic TIFF is supported; BigTIFF, TIFF files with SubIFDs and BMP files wrapping JPEG/PNG are refused. This checks image geometry without decoding the complete raster.

Changes remain pending until you select **Save**. **Reset** only cancels pending changes.

### Look up tags online

**Search online** searches MusicBrainz without a key. To include Discogs, open **Discogs token** in the search pop-in, enter a personal access token from Discogs settings and select **Save token**. The saved token lives in macOS Keychain, Windows Credential Manager or Linux Secret Service; the input never displays an existing secret. Linux needs an unlocked Secret Service provider.

**Forget token** removes the app's saved credential. To revoke the token itself, use Discogs settings. MusicBrainz remains available if the credential store is locked or unavailable.

Tokens saved by older versions migrate automatically at startup. A failed migration keeps a recovery copy only in memory for the current session: unlock the system store and select **Retry secure storage** before closing the app. If browser cleanup fails, the panel warns that an old plaintext copy may remain; **Forget token** stays incomplete until cleanup succeeds. Removing the active browser entry cannot erase copies in older backups.

## Convert a selection

The two-arrow button in the top bar opens the **Convert** panel on the right.

Drop files or a folder directly on this panel to make an independent conversion list. If tracks are already analyzed and selected, use **Add selected** to add them to that list.

Choose the output format, then the options available for it. Lossy formats expose a bitrate; FLAC exposes encoding effort. **Also copy other files** keeps covers, playlists and other non-audio files alongside the converted tracks.

When the list and settings are right, select **Convert** and choose a destination folder. FlacCompagnon leaves the analysis list intact: conversion does not erase or replace the originals. Choose an empty folder for a new conversion: existing output files are preserved, duplicate output names are rejected and symbolic links cannot redirect output outside the chosen folder. **Also copy other files** also preserves existing outputs and skips the output folder when it is inside the source folder.

## Listen to a track

The bottom bar is the player. Click a row’s play button, or select tracks and use the main button in the bar.

The playback queue is captured when playback starts: several selected tracks play in display order; one selected track starts there and continues down the list; no selection starts at the top. Changing the selection while playing does not change that queue. Previous and next follow the captured queue. Use the slider for volume and the progress bar to move through the track.

Listening is particularly useful after a finding: locate the section an analysis highlights, then compare it with what you actually hear.

## Search a library

The **Filter…** field in the upper left narrows the list as you type. It searches the file name, information shown by the analysis and tags already read: artist, album, title, genre, catalogue number and more.

Enter several words to keep tracks containing each word in one piece of information. Clear the field to restore the full list. This filter does not remove a file from an export or report; it only changes the display and playback queue.

## Save and resume later

Use **Save…** to save a report. The JSON keeps analysis results and the displayed track order; you can drop it back into FlacCompagnon later without running the analysis again.

For automated use, see [Using the CLI](cli.md). It provides the same analyses from a terminal and can write the same JSON format.

Imported reports must be regular files no larger than 64 MiB. Report, playlist and artwork exports replace their destination atomically and reject symbolic links.

If tracks are missing, use the presence check, then the **Locate** folder button that appears, to search another folder. Matching uses file names and folder suffixes, not audio fingerprints; equal-ranking candidates remain missing. Reanalyze relocated files before trusting their old measurements.
