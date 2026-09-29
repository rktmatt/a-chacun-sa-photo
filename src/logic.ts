// Règles pures de l'application (sans Vue, DOM ni Tauri), testées dans logic.test.ts.

export type Child = { id: number; name: string; allowsDistribution: boolean };
export type Tag = { id: string; childId: number; x: number; y: number; width: number; height: number };
export type Box = Pick<Tag, "x" | "y" | "width" | "height">;
export type Handle = "move" | "nw" | "ne" | "sw" | "se";

// Visage à flouter partout (enfant d'une autre classe, passant…) : jamais de dossier, jamais d'accord.
export const UNKNOWN_ID = 0;
export const unknownChild: Child = { id: UNKNOWN_ID, name: "Visage inconnu", allowsDistribution: false };
export const MIN_TAG_SIZE = 0.015;
// Marge autour de l'ellipse de masquage, en fraction de la largeur/hauteur de la photo, de chaque côté.
export const MASK_PADDING = 0.015;

export const findChild = (roster: Child[], id: number) => id === UNKNOWN_ID ? unknownChild : roster.find((child) => child.id === id);

// Dans le dossier de `recipient`, un cadre est masqué sauf s'il s'agit de l'enfant lui-même ou d'un enfant autorisé.
// Un cadre dont l'élève est introuvable est traité comme non autorisé.
export function isMaskedFor(tag: Tag, recipient: Child, roster: Child[]) {
  const child = findChild(roster, tag.childId);
  return !(child?.id === recipient.id || child?.allowsDistribution);
}

// Ellipse inscrite dans le cadre (marge de 1,5 % de chaque côté), calculée avant tout rognage aux bords,
// et rectangle en pixels qui la contient, rogné à la photo. `null` si le cadre est hors de la photo.
export function maskEllipse(tag: Box, width: number, height: number) {
  const cx = (tag.x + tag.width / 2) * width;
  const cy = (tag.y + tag.height / 2) * height;
  const rx = (tag.width / 2 + MASK_PADDING) * width;
  const ry = (tag.height / 2 + MASK_PADDING) * height;
  const x = Math.max(0, Math.floor(cx - rx));
  const y = Math.max(0, Math.floor(cy - ry));
  const w = Math.min(width, Math.ceil(cx + rx)) - x;
  const h = Math.min(height, Math.ceil(cy + ry)) - y;
  return w > 0 && h > 0 ? { cx, cy, rx, ry, x, y, w, h } : null;
}

// Une photo par élève de la classe dans laquelle il a un cadre ; « Visage inconnu » n'est pas dans la classe, donc jamais exporté.
export function exportJobs<P extends { tags: Tag[] }>(roster: Child[], photos: P[]) {
  return roster.flatMap((recipient) =>
    photos.filter((photo) => photo.tags.some((tag) => tag.childId === recipient.id)).map((photo, index) => ({ recipient, photo, index })));
}

// Les cadres d'un élève retiré de la classe restent floutés : ils deviennent des visages inconnus.
export function forgetTags(photos: { tags: Tag[] }[], ids: number[]) {
  for (const photo of photos) for (const tag of photo.tags) if (ids.includes(tag.childId)) tag.childId = UNKNOWN_ID;
}

export const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(max, value));

// Déplacement ou redimensionnement d'un cadre (coordonnées normalisées 0–1), toujours dans la photo.
export function editBox(origin: Box, handle: Handle, dx: number, dy: number): Box {
  if (handle === "move") {
    return { ...origin, x: clamp(origin.x + dx, 0, 1 - origin.width), y: clamp(origin.y + dy, 0, 1 - origin.height) };
  }
  let left = origin.x, top = origin.y, right = origin.x + origin.width, bottom = origin.y + origin.height;
  if (handle.includes("w")) left = clamp(left + dx, 0, right - MIN_TAG_SIZE);
  if (handle.includes("e")) right = clamp(right + dx, left + MIN_TAG_SIZE, 1);
  if (handle.includes("n")) top = clamp(top + dy, 0, bottom - MIN_TAG_SIZE);
  if (handle.includes("s")) bottom = clamp(bottom + dy, top + MIN_TAG_SIZE, 1);
  return { x: left, y: top, width: right - left, height: bottom - top };
}

export function decodeText(buffer: ArrayBuffer | Uint8Array) {
  try { return new TextDecoder("utf-8", { fatal: true }).decode(buffer); } catch { return new TextDecoder("windows-1252").decode(buffer); }
}

// Liste d'élèves : un nom par ligne, ou export tableur/ONDE avec colonnes « Nom » et « Prénom ».
export function parseNames(text: string): string[] {
  const lines = text.replace(/^﻿/, "").split(/\r?\n/).filter((line) => line.trim());
  if (!lines.length) return [];
  const separator = [";", "\t", ","].find((candidate) => lines[0].includes(candidate));
  const cells = (line: string) => (separator ? line.split(separator) : [line]).map((cell) => cell.trim().replace(/^"(.*)"$/, "$1").trim());
  const header = cells(lines[0]).map((cell) => cell.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase());
  const first = header.findIndex((cell) => cell.startsWith("prenom"));
  const last = header.findIndex((cell) => cell.startsWith("nom"));
  const hasHeader = first >= 0 || last >= 0;
  const names = (hasHeader ? lines.slice(1) : lines).map(cells).map((row) =>
    first >= 0 && last >= 0 ? `${row[first] ?? ""} ${row[last] ?? ""}` : row[first >= 0 ? first : last >= 0 ? last : 0] ?? "");
  return [...new Set(names.map((name) => name.replace(/\s+/g, " ").trim()).filter(Boolean))];
}

export function cleanFolderName(name: string) { return name.replace(/[\\/:*?"<>|]/g, "-").trim() || "élève"; }
export function jpegName(name: string) { return `${name.replace(/\.[^.]+$/, "") || "photo"}.jpg`; }

export function exportFolderName(date: Date) {
  const two = (value: number) => String(value).padStart(2, "0");
  return `Photos du ${two(date.getDate())}-${two(date.getMonth() + 1)}-${date.getFullYear()} à ${two(date.getHours())}h${two(date.getMinutes())}`;
}
