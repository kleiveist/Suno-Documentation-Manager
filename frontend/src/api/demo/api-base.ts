import { subscriptionEvidenceRelevance } from "../../domain/workflow";
import { emptyAudioScreeningSettings, emptyProfile, emptyTimestampSettings } from "../../domain/types";
import type {
  ActionResult,
  AudioScreeningSettings,
  EvidenceItem,
  GlobalEvidenceItem,
  GlobalProfile,
  TimestampSettings,
  TrackDetail,
  TrackLibraryAssignment,
  WorkspaceSummary
} from "../../domain/types";
import { markAudioScreeningStale, releaseAudio } from "./audio-screening";
import { refresh } from "./fixtures";
import { clone, evidence } from "./helpers";
import { attachConfiguredTimestamp } from "./timestamp";

interface DemoState {
  workspace: WorkspaceSummary | null;
  profile: GlobalProfile;
  tracks: Map<string, TrackDetail>;
  albums: Map<string, string>;
  globalEvidence: GlobalEvidenceItem[];
  timestampSettings: TimestampSettings;
  timestampSecretConfigured: boolean;
  audioScreeningSettings: AudioScreeningSettings;
  audioAccessKeyConfigured: boolean;
  audioAccessSecretConfigured: boolean;
}

function initialDemoState(): DemoState {
  return {
    workspace: null,
    profile: {
      ...emptyProfile,
      artistName: "GRAV0ID",
      sunoProfileName: "Grav0id Studio",
      sunoHandle: "@grav0id",
      sunoPlan: "Premier",
      subscriptionStartDate: "2026-01-01",
      defaultAiImageService: "OpenAI"
    },
    tracks: new Map<string, TrackDetail>(),
    albums: new Map<string, string>(),
    globalEvidence: [
      {
        ...evidence("subscription_payment", "subscription_2026-07.pdf"),
        coverageStart: "2026-07-01",
        coverageEnd: "2026-07-31"
      }
    ],
    timestampSettings: clone(emptyTimestampSettings),
    timestampSecretConfigured: false,
    audioScreeningSettings: clone(emptyAudioScreeningSettings),
    audioAccessKeyConfigured: false,
    audioAccessSecretConfigured: false
  };
}

const DEMO_STATE = Symbol();

export abstract class DemoApiBase {
  readonly mode = "demo" as const;

  private readonly [DEMO_STATE] = initialDemoState();

  protected get state(): DemoState {
    return this[DEMO_STATE];
  }

  protected attachGlobalToTrack(track: TrackDetail, item: GlobalEvidenceItem): void {
    if (track.evidence.some((entry) => entry.sourceGlobalEvidenceId === item.id)) return;
    if (item.role === "subscription_payment" && !subscriptionEvidenceRelevance(item, track.fields).relevant) {
      throw new Error(
        "Der ausgewählte Abo-Nachweis überschneidet weder den Produktionszeitraum noch deckt er die Finalgeneration ab."
      );
    }
    track.evidence.push({
      ...clone(item),
      id: crypto.randomUUID(),
      sourceGlobalEvidenceId: item.id,
      provenance: "global_copy",
      relativePath: `04_LICENSES/${item.role === "suno_terms_rights" ? "suno_terms" : "subscription"}_${item.fileName}`
    });
    if (item.role === "suno_terms_rights") track.fields.sunoTermsEvidenceNotAvailable = false;
    refresh(track);
  }

  protected albumKey(title: string): string {
    return title.trim().normalize("NFKC").toLocaleLowerCase("de-DE");
  }

  protected rememberAlbum(library: TrackLibraryAssignment): void {
    if (library.section === "album" && library.albumTitle) {
      this.state.albums.set(this.albumKey(library.albumTitle), library.albumTitle.trim());
    }
  }

  protected albumList(): string[] {
    return [...this.state.albums.values()].sort((left, right) =>
      left.localeCompare(right, "de", { sensitivity: "base", numeric: true })
    );
  }

  protected get(trackId: string): TrackDetail {
    const track = this.state.tracks.get(trackId);
    if (!track) throw new Error("Der Track wurde im aktuellen Workspace nicht gefunden.");
    return track;
  }

  protected mutableTrack(trackId: string): TrackDetail {
    const track = this.get(trackId);
    if (track.status === "FINALIZED") {
      throw new Error("Der Track ist finalisiert. Lege vor Änderungen eine neue Revision an.");
    }
    if (track.status === "SUPERSEDED") {
      throw new Error("Der Track wurde durch eine neuere Revision ersetzt und kann nicht mehr geändert werden.");
    }
    return track;
  }

  protected result(track: TrackDetail, message: string): ActionResult {
    return { message, track: clone(track) };
  }

  protected attachConfiguredTimestamp(track: TrackDetail): TrackDetail {
    return attachConfiguredTimestamp(track, this.state.timestampSettings);
  }

  protected releaseAudio(track: TrackDetail): EvidenceItem | undefined {
    return releaseAudio(track);
  }

  protected markAudioScreeningStale(track: TrackDetail): void {
    markAudioScreeningStale(track);
  }
}
