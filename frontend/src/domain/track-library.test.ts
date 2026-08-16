import { describe, expect, it } from "vitest";

import {
  groupTrackLibrary,
  normalizedTrackLibrary,
  trackLibraryAssignment,
  type TrackLibrarySource
} from "./track-library";

const track = (
  id: string,
  title: string,
  library?: TrackLibrarySource["library"],
  status: TrackLibrarySource["status"] = "ACTIVE"
): TrackLibrarySource => ({
  id,
  title,
  relativePath: title.toLocaleLowerCase("de-DE").replaceAll(" ", "-"),
  status,
  library
});

describe("track library grouping", () => {
  it("always returns Albums and Singles as the two top-level groups", () => {
    expect(groupTrackLibrary([])).toEqual({ albums: [], singles: [] });
  });

  it("keeps physical albums visible before their first track is created", () => {
    expect(groupTrackLibrary([], {}, ["  Gravity Drift  ", "Gravity Drift", "Second Album"]))
      .toEqual({
        albums: [
          { title: "Gravity Drift", tracks: [] },
          { title: "Second Album", tracks: [] }
        ],
        singles: []
      });
    expect(groupTrackLibrary([], { query: "second" }, ["Gravity Drift", "Second Album"]).albums)
      .toEqual([{ title: "Second Album", tracks: [] }]);
  });

  it("does not load hidden folders or tracks into the rendered library", () => {
    const grouped = groupTrackLibrary([
      {
        ...track("hidden-album", "Archived", { section: "album", albumTitle: ".archive" }),
        relativePath: ".archive/Archived"
      },
      {
        ...track("hidden-track", "Draft", { section: "album", albumTitle: "Visible" }),
        relativePath: "Visible/.draft"
      },
      track("visible", "Current", { section: "album", albumTitle: "Visible" })
    ], {}, [".archive", ".cache", "Visible"]);

    expect(grouped.albums).toEqual([
      { title: "Visible", tracks: [expect.objectContaining({ id: "visible" })] }
    ]);
    expect(grouped.singles).toEqual([]);
  });

  it("groups album names case-insensitively and sorts albums and tracks", () => {
    const grouped = groupTrackLibrary([
      track("4", "Zulu", { section: "album", albumTitle: "neon nights" }),
      track("3", "Alpha", { section: "album", albumTitle: " Neon Nights " }),
      track("2", "Beta", { section: "album", albumTitle: "Another Album" }),
      track("1", "Single Z", { section: "single" }),
      track("0", "Single A", { section: "single" })
    ]);

    expect(grouped.albums.map((album) => album.title)).toEqual(["Another Album", "neon nights"]);
    expect(grouped.albums[1].tracks.map((item) => item.title)).toEqual(["Alpha", "Zulu"]);
    expect(grouped.singles.map((item) => item.title)).toEqual(["Single A", "Single Z"]);
  });

  it("places legacy and invalid album assignments in Singles without losing tracks", () => {
    const inputs = [
      track("legacy", "Legacy"),
      track("invalid", "Missing album", { section: "album" }),
      track("album", "Album track", { section: "album", albumTitle: "First" }),
      track("single", "Single track", { section: "single", albumTitle: "Ignored" })
    ];
    const grouped = groupTrackLibrary(inputs);
    const ids = [
      ...grouped.albums.flatMap((album) => album.tracks.map((item) => item.id)),
      ...grouped.singles.map((item) => item.id)
    ];

    expect(ids.sort()).toEqual(inputs.map((item) => item.id).sort());
    expect(new Set(ids).size).toBe(inputs.length);
    expect(grouped.singles.map((item) => item.id)).toEqual(["legacy", "invalid", "single"]);
    expect(normalizedTrackLibrary(undefined)).toEqual({ section: "single" });
  });

  it("searches track paths and album titles while retaining the hierarchy", () => {
    const inputs = [
      track("one", "Opening", { section: "album", albumTitle: "Northern Lights" }),
      track("two", "Finale", { section: "album", albumTitle: "Northern Lights" }),
      track("three", "Elsewhere", { section: "album", albumTitle: "Southern Lights" }),
      { ...track("single", "Solo", { section: "single" }), relativePath: "special-path" }
    ];

    const byAlbum = groupTrackLibrary(inputs, { query: "NORTHERN" });
    expect(byAlbum.albums).toHaveLength(1);
    expect(byAlbum.albums[0].tracks.map((item) => item.id)).toEqual(["two", "one"]);
    expect(byAlbum.singles).toEqual([]);

    const byPath = groupTrackLibrary(inputs, { query: "special" });
    expect(byPath.albums).toEqual([]);
    expect(byPath.singles.map((item) => item.id)).toEqual(["single"]);
  });

  it("applies the status filter inside album and single groups", () => {
    const grouped = groupTrackLibrary([
      track("album-open", "Open", { section: "album", albumTitle: "Record" }, "DRAFT"),
      track("album-ready", "Ready", { section: "album", albumTitle: "Record" }, "READY"),
      track("single-open", "Single Open", { section: "single" }, "ACTIVE"),
      track("single-final", "Single Final", { section: "single" }, "FINALIZED")
    ], { status: "open" });

    expect(grouped.albums[0].tracks.map((item) => item.id)).toEqual(["album-open"]);
    expect(grouped.singles.map((item) => item.id)).toEqual(["single-open"]);
  });
});

describe("track library assignment", () => {
  it("requires an album title only for album tracks and drops it for singles", () => {
    expect(trackLibraryAssignment("album", "  New Record  ")).toEqual({
      section: "album",
      albumTitle: "New Record"
    });
    expect(trackLibraryAssignment("album", "   ")).toBeNull();
    expect(trackLibraryAssignment("single", "Old Record")).toEqual({ section: "single" });
    expect(trackLibraryAssignment("unknown", "Record")).toBeNull();
  });

  it("matches the native album-folder validation boundary", () => {
    expect(trackLibraryAssignment("album", "a".repeat(200))).toEqual({
      section: "album",
      albumTitle: "a".repeat(200)
    });
    expect(trackLibraryAssignment("album", "a".repeat(201))).toBeNull();
    expect(trackLibraryAssignment("album", ` ${"a".repeat(200)} `)).toEqual({
      section: "album",
      albumTitle: "a".repeat(200)
    });
    expect(trackLibraryAssignment("album", "Invalid\nAlbum")).toBeNull();
    expect(trackLibraryAssignment("album", "unsafe/name")).toBeNull();
    expect(trackLibraryAssignment("album", "Singles")).toBeNull();
    expect(trackLibraryAssignment("album", ".archive")).toBeNull();
    expect(trackLibraryAssignment("album", "  .private  ")).toBeNull();
    expect(trackLibraryAssignment("single", "Ignored\nAlbum")).toEqual({ section: "single" });
  });
});
