import type { OperationProgressHandler } from "../desktop";
import { demoProgress, evidence, managedArtworkName, now, wait } from "./helpers";
import { refresh } from "./fixtures";
import { DemoLibraryApi } from "./api-library";

export abstract class DemoDocumentsApi extends DemoLibraryApi {
  async generateDocuments(trackId: string, _adoptExisting: boolean, onProgress: OperationProgressHandler) {
    const track = this.mutableTrack(trackId);
    const documentFiles = [
      "02_SUNO/suno_project.txt",
      "02_SUNO/Lyrics.md",
      "02_SUNO/Style.md",
      "03_DOCUMENTATION/README.md",
      "03_DOCUMENTATION/AI_USAGE.md",
      "04_LICENSES/suno_account_and_license.md",
      "04_LICENSES/openai_image_generation.md",
      "05_ARTWORK/artwork_process.md"
    ];
    await demoProgress(onProgress, [
      {
        stage: "preparing_documents",
        processedBytes: 0,
        totalBytes: 0,
        processedFiles: 0,
        totalFiles: documentFiles.length
      },
      {
        stage: "rendering_documents",
        processedBytes: 0,
        totalBytes: 0,
        processedFiles: 0,
        totalFiles: documentFiles.length
      },
      ...documentFiles.map((currentFile, index) => ({
        stage: "writing_documents",
        processedBytes: 0,
        totalBytes: 0,
        processedFiles: index + 1,
        totalFiles: documentFiles.length,
        currentFile
      })),
      {
        stage: "complete",
        processedBytes: 0,
        totalBytes: 0,
        processedFiles: documentFiles.length,
        totalFiles: documentFiles.length
      }
    ]);
    track.documents = {
      generated: true,
      current: true,
      generatedAt: now(),
      templateVersion: "1.11",
      files: documentFiles
    };
    track.integrity.generated = false;
    track.integrity.verified = false;
    refresh(track);
    return this.result(track, "8 Dokumente wurden deterministisch erzeugt.");
  }

  async generateArtworkDisclosure(trackId: string, disclosureText: string) {
    await wait();
    const track = this.mutableTrack(trackId);
    track.fields.disclosureApplied = true;
    if (disclosureText) track.fields.disclosureText = disclosureText;
    const source = [...track.evidence].reverse().find((item) => item.role === "ai_artwork_original" && item.verified);
    if (!source) throw new Error("Importiere zuerst das unveränderte KI-Artwork.");
    if (!track.evidence.some((item) => item.role === "ai_artwork_edited")) {
      track.evidence.push({
        ...evidence("ai_artwork_edited", managedArtworkName(track.title, "ai_artwork_edited", "png")!),
        provenance: "generated_disclosure",
        derivedFromEvidenceId: source.id,
        generatorVersion: "local-disclosure-v1",
        generatedDisclosureText: track.fields.disclosureText.trim()
      });
    }
    track.documents.current = false;
    refresh(track);
    return this.result(track, "Der sichtbare KI-Hinweis wurde lokal auf einer neuen Artwork-Version angewendet.");
  }

  async calculateHashes(trackId: string, onProgress: OperationProgressHandler) {
    const track = this.mutableTrack(trackId);
    if (!track.documents.current) throw new Error("Erzeuge zuerst die aktuellen Dokumente.");
    const totalFiles = track.evidence.length + track.documents.files.length;
    const totalBytes = Math.max(totalFiles, 1) * 8476231;
    await demoProgress(onProgress, [
      { stage: "discovering_files", processedBytes: 0, totalBytes: 0, processedFiles: 0, totalFiles: 0 },
      {
        stage: "hashing",
        processedBytes: Math.round(totalBytes * 0.25),
        totalBytes,
        processedFiles: Math.floor(totalFiles * 0.25),
        totalFiles,
        currentFile: "01_RELEASE/demo.wav"
      },
      {
        stage: "hashing",
        processedBytes: Math.round(totalBytes * 0.7),
        totalBytes,
        processedFiles: Math.floor(totalFiles * 0.7),
        totalFiles,
        currentFile: "02_SUNO/demo.zip"
      },
      {
        stage: "writing_hash_list",
        processedBytes: totalBytes,
        totalBytes,
        processedFiles: totalFiles,
        totalFiles,
        currentFile: "03_DOCUMENTATION/SHA256SUMS.txt"
      },
      { stage: "verifying", processedBytes: totalBytes, totalBytes, processedFiles: totalFiles, totalFiles },
      { stage: "complete", processedBytes: totalBytes, totalBytes, processedFiles: totalFiles, totalFiles }
    ]);
    track.integrity = {
      generated: true,
      verified: false,
      fileCount: track.evidence.length + track.documents.files.length,
      verifiedCount: 0,
      generatedAt: now(),
      mismatchFiles: []
    };
    refresh(track);
    return this.result(track, `${track.integrity.fileCount} Dateien wurden gehasht.`);
  }

  async verifyHashes(trackId: string, onProgress: OperationProgressHandler) {
    const track = this.get(trackId);
    if (!track.integrity.generated) throw new Error("Erzeuge zuerst SHA-256-Prüfsummen.");
    const totalFiles = track.integrity.fileCount;
    const totalBytes = Math.max(totalFiles, 1) * 8476231;
    await demoProgress(onProgress, [
      { stage: "reading_hash_list", processedBytes: 0, totalBytes: 0, processedFiles: 0, totalFiles },
      {
        stage: "verifying",
        processedBytes: Math.round(totalBytes * 0.35),
        totalBytes,
        processedFiles: Math.floor(totalFiles * 0.35),
        totalFiles,
        currentFile: "01_RELEASE/demo.wav"
      },
      {
        stage: "verifying",
        processedBytes: Math.round(totalBytes * 0.8),
        totalBytes,
        processedFiles: Math.floor(totalFiles * 0.8),
        totalFiles,
        currentFile: "02_SUNO/demo.zip"
      },
      { stage: "comparing_hashes", processedBytes: totalBytes, totalBytes, processedFiles: totalFiles, totalFiles },
      { stage: "complete", processedBytes: totalBytes, totalBytes, processedFiles: totalFiles, totalFiles }
    ]);
    track.integrity.verified = true;
    track.integrity.verifiedCount = track.integrity.fileCount;
    track.integrity.verifiedAt = now();
    track.integrity.mismatchFiles = [];
    refresh(track);
    return this.result(
      track,
      `${track.integrity.verifiedCount} von ${track.integrity.fileCount} Dateien erfolgreich verifiziert.`
    );
  }
}
