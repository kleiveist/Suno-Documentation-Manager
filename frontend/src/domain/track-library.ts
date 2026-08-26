import type { TrackLibraryAssignment, TrackStatus } from "./types";

export type TrackLibraryStatusFilter = "all" | "open" | "ready" | "finalized";

export interface TrackLibrarySource {
  id: string;
  title: string;
  relativePath: string;
  status: TrackStatus;
  library?: TrackLibraryAssignment;
}

export interface AlbumTrackGroup<T extends TrackLibrarySource> {
  title: string;
  tracks: T[];
}

export interface GroupedTrackLibrary<T extends TrackLibrarySource> {
  albums: AlbumTrackGroup<T>[];
  singles: T[];
}

export interface TrackLibraryFilters {
  query?: string;
  status?: TrackLibraryStatusFilter;
}

const germanCollator = new Intl.Collator("de", { sensitivity: "base", numeric: true });

function normalizedText(value: string): string {
  return value.trim().normalize("NFKC").toLocaleLowerCase("de-DE");
}

function compareTracks(left: TrackLibrarySource, right: TrackLibrarySource): number {
  return (
    germanCollator.compare(left.title, right.title) ||
    germanCollator.compare(left.relativePath, right.relativePath) ||
    left.id.localeCompare(right.id)
  );
}

function matchesStatus(status: TrackStatus, filter: TrackLibraryStatusFilter): boolean {
  return (
    filter === "all" ||
    (filter === "open" && (status === "DRAFT" || status === "ACTIVE")) ||
    (filter === "ready" && status === "READY") ||
    (filter === "finalized" && status === "FINALIZED")
  );
}

function matchesTrackQuery(track: TrackLibrarySource, query: string): boolean {
  return !query || normalizedText(track.title).includes(query) || normalizedText(track.relativePath).includes(query);
}

function startsWithHiddenDirectory(value: string): boolean {
  return value.trim().startsWith(".");
}

function hasHiddenPathComponent(relativePath: string): boolean {
  return relativePath
    .replaceAll("\\", "/")
    .split("/")
    .some((component) => component.startsWith("."));
}

function addDeclaredAlbum<T extends TrackLibrarySource>(
  albums: Map<string, AlbumTrackGroup<T>>,
  rawTitle: string
): void {
  const title = rawTitle.trim();
  if (!title || startsWithHiddenDirectory(title)) return;
  const key = normalizedText(title);
  if (!albums.has(key)) albums.set(key, { title, tracks: [] });
}

function hiddenTrack(track: TrackLibrarySource): boolean {
  return (
    hasHiddenPathComponent(track.relativePath) ||
    (track.library?.section === "album" && startsWithHiddenDirectory(track.library.albumTitle ?? ""))
  );
}

function albumGroup<T extends TrackLibrarySource>(
  albums: Map<string, AlbumTrackGroup<T>>,
  title: string
): AlbumTrackGroup<T> {
  const key = normalizedText(title);
  const existing = albums.get(key);
  if (existing) return existing;
  const created = { title, tracks: [] as T[] };
  albums.set(key, created);
  return created;
}

function classifyVisibleTrack<T extends TrackLibrarySource>(
  track: T,
  status: TrackLibraryStatusFilter,
  query: string,
  albums: Map<string, AlbumTrackGroup<T>>,
  singles: T[]
): void {
  if (hiddenTrack(track) || !matchesStatus(track.status, status)) return;
  const library = normalizedTrackLibrary(track.library);
  if (library.section === "single") {
    if (matchesTrackQuery(track, query)) singles.push(track);
    return;
  }

  const title = library.albumTitle!;
  const group = albumGroup(albums, title);
  if (!query || normalizedText(title).includes(query) || matchesTrackQuery(track, query)) {
    group.tracks.push(track);
  }
}

function visibleAlbum<T extends TrackLibrarySource>(group: AlbumTrackGroup<T>, query: string): boolean {
  return !query || normalizedText(group.title).includes(query) || group.tracks.length > 0;
}

/**
 * Normalizes persisted and legacy values for presentation. Invalid or absent
 * album assignments remain visible by falling back to the Singles section.
 */
export function normalizedTrackLibrary(library: TrackLibraryAssignment | undefined): TrackLibraryAssignment {
  const albumTitle = library?.albumTitle?.trim();
  return library?.section === "album" && albumTitle ? { section: "album", albumTitle } : { section: "single" };
}

/**
 * Produces both permanent top-level library sections. Each non-hidden input
 * track is classified exactly once before search and status filters are applied.
 */
export function groupTrackLibrary<T extends TrackLibrarySource>(
  tracks: readonly T[],
  filters: TrackLibraryFilters = {},
  albumTitles: readonly string[] = []
): GroupedTrackLibrary<T> {
  const query = normalizedText(filters.query ?? "");
  const status = filters.status ?? "all";
  const singles: T[] = [];
  const albums = new Map<string, AlbumTrackGroup<T>>();

  for (const rawTitle of albumTitles) addDeclaredAlbum(albums, rawTitle);

  for (const track of tracks) {
    classifyVisibleTrack(track, status, query, albums, singles);
  }

  return {
    albums: [...albums.values()]
      .filter((group) => visibleAlbum(group, query))
      .map((group) => ({ ...group, tracks: group.tracks.sort(compareTracks) }))
      .sort((left, right) => germanCollator.compare(left.title, right.title)),
    singles: singles.sort(compareTracks)
  };
}

export function trackLibraryAssignment(section: string, albumTitle: string): TrackLibraryAssignment | null {
  if (section === "single") return { section: "single" };
  if ([...albumTitle].some((character) => /\p{Cc}/u.test(character)) || /[\\/]/.test(albumTitle)) {
    return null;
  }
  const normalizedTitle = albumTitle.trim();
  if (
    section === "album" &&
    normalizedTitle &&
    !normalizedTitle.startsWith(".") &&
    normalizedTitle.toLocaleLowerCase("de-DE") !== "singles" &&
    [...normalizedTitle].length <= 200
  ) {
    return { section: "album", albumTitle: normalizedTitle };
  }
  return null;
}
