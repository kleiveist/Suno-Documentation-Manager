import type {
  TrackLibraryAssignment,
  TrackStatus
} from "./types";

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
  return germanCollator.compare(left.title, right.title)
    || germanCollator.compare(left.relativePath, right.relativePath)
    || left.id.localeCompare(right.id);
}

function matchesStatus(status: TrackStatus, filter: TrackLibraryStatusFilter): boolean {
  return filter === "all"
    || (filter === "open" && (status === "DRAFT" || status === "ACTIVE"))
    || (filter === "ready" && status === "READY")
    || (filter === "finalized" && status === "FINALIZED");
}

function matchesTrackQuery(track: TrackLibrarySource, query: string): boolean {
  return !query
    || normalizedText(track.title).includes(query)
    || normalizedText(track.relativePath).includes(query);
}

/**
 * Normalizes persisted and legacy values for presentation. Invalid or absent
 * album assignments remain visible by falling back to the Singles section.
 */
export function normalizedTrackLibrary(
  library: TrackLibraryAssignment | undefined
): TrackLibraryAssignment {
  const albumTitle = library?.albumTitle?.trim();
  return library?.section === "album" && albumTitle
    ? { section: "album", albumTitle }
    : { section: "single" };
}

/**
 * Produces both permanent top-level library sections. Each input track is
 * classified exactly once before search and status filters are applied.
 */
export function groupTrackLibrary<T extends TrackLibrarySource>(
  tracks: readonly T[],
  filters: TrackLibraryFilters = {}
): GroupedTrackLibrary<T> {
  const query = normalizedText(filters.query ?? "");
  const status = filters.status ?? "all";
  const singles: T[] = [];
  const albums = new Map<string, AlbumTrackGroup<T>>();

  for (const track of tracks) {
    if (!matchesStatus(track.status, status)) continue;
    const library = normalizedTrackLibrary(track.library);
    if (library.section === "single") {
      if (matchesTrackQuery(track, query)) singles.push(track);
      continue;
    }

    const title = library.albumTitle!;
    const key = normalizedText(title);
    let group = albums.get(key);
    if (!group) {
      group = { title, tracks: [] };
      albums.set(key, group);
    }
    if (!query || key.includes(query) || matchesTrackQuery(track, query)) {
      group.tracks.push(track);
    }
  }

  return {
    albums: [...albums.values()]
      .filter((group) => group.tracks.length > 0)
      .map((group) => ({ ...group, tracks: group.tracks.sort(compareTracks) }))
      .sort((left, right) => germanCollator.compare(left.title, right.title)),
    singles: singles.sort(compareTracks)
  };
}

export function trackLibraryAssignment(
  section: string,
  albumTitle: string
): TrackLibraryAssignment | null {
  if (section === "single") return { section: "single" };
  if ([...albumTitle].some((character) => /\p{Cc}/u.test(character))) {
    return null;
  }
  const normalizedTitle = albumTitle.trim();
  if (section === "album" && normalizedTitle && [...normalizedTitle].length <= 200) {
    return { section: "album", albumTitle: normalizedTitle };
  }
  return null;
}
