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

To understand a column, open its article in [Analyses](index.md). Every article explains what the measurement observes and its limitations.

## Edit tags and artwork

Select one or more tracks in the list. The **Tags** panel opens on the left.

It contains the album artwork and the common fields: title, artist, album, album artist, year, genre, track number and comment. Edit one or more fields, then use **Save** at the bottom of the panel to write the changes to the selected files.

Artwork follows the role chosen in the list beneath the cover, such as _Front cover_ or _Back cover_. Change the role to display the matching image. You can:

- drop an image on the cover to stage it for the selected tracks;
- use the export button below the image to extract it next to the audio files;
- use the trash button to stage its deletion.

Changes remain pending until you select **Save**. **Reset** only cancels pending changes.

## Convert a selection

The two-arrow button in the top bar opens the **Convert** panel on the right.

Drop files or a folder directly on this panel to make an independent conversion list. If tracks are already analyzed and selected, use **Add selected** to add them to that list.

Choose the output format, then the options available for it. Lossy formats expose a bitrate; FLAC exposes encoding effort. **Also copy other files** keeps covers, playlists and other non-audio files alongside the converted tracks.

When the list and settings are right, select **Convert** and choose a destination folder. FlacCompagnon leaves the analysis list intact: conversion does not erase or replace the originals.

## Listen to a track

The bottom bar is the player. Click a row’s play button, or select tracks and use the main button in the bar.

Previous and next follow the selection when there is one. Without a selection, they follow the currently displayed list. Use the slider for volume and the progress bar to move through the track.

Listening is particularly useful after a finding: locate the section an analysis highlights, then compare it with what you actually hear.

## Search a library

The **Filter…** field in the upper left narrows the list as you type. It searches the file name, information shown by the analysis and tags already read: artist, album, title, genre, catalogue number and more.

Enter several words to keep tracks containing each word in one piece of information. Clear the field to restore the full list. This filter does not remove a file from an export or report; it only changes the display and playback queue.

## Save and resume later

Use **Save…** to save a report. The JSON keeps analysis results and the displayed track order; you can drop it back into FlacCompagnon later without running the analysis again.

For automated use, see [Using the CLI](cli.md). It provides the same analyses from a terminal and can write the same JSON format.
