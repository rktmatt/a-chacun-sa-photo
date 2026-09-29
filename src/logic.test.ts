import { describe, expect, it } from "vitest";
import {
  type Child, type Tag, MASK_PADDING, MIN_TAG_SIZE, UNKNOWN_ID, cleanFolderName, decodeText, editBox, exportFolderName,
  exportJobs, findChild, forgetTags, isMaskedFor, jpegName, maskEllipse, parseNames,
} from "./logic";

const alice: Child = { id: 1, name: "Alice", allowsDistribution: false };
const bruno: Child = { id: 2, name: "Bruno", allowsDistribution: true };
const chloe: Child = { id: 3, name: "Chloé", allowsDistribution: false };
const roster = [alice, bruno, chloe];
const tag = (childId: number, box: Partial<Tag> = {}): Tag => ({ id: `t${childId}`, childId, x: 0.4, y: 0.4, width: 0.2, height: 0.2, ...box });

describe("isMaskedFor : qui est flouté dans quel dossier", () => {
  it("ne floute jamais l'enfant dans son propre dossier, même sans accord", () => {
    expect(isMaskedFor(tag(alice.id), alice, roster)).toBe(false);
  });
  it("floute un autre enfant sans accord", () => {
    expect(isMaskedFor(tag(chloe.id), alice, roster)).toBe(true);
    expect(isMaskedFor(tag(alice.id), bruno, roster)).toBe(true);
  });
  it("laisse net un enfant avec accord", () => {
    expect(isMaskedFor(tag(bruno.id), alice, roster)).toBe(false);
  });
  it("floute toujours un visage inconnu", () => {
    for (const recipient of roster) expect(isMaskedFor(tag(UNKNOWN_ID), recipient, roster)).toBe(true);
  });
  it("floute un cadre dont l'élève n'existe plus", () => {
    expect(isMaskedFor(tag(99), alice, roster)).toBe(true);
    expect(isMaskedFor(tag(bruno.id), alice, [alice])).toBe(true);
  });
  it("suit un retrait d'accord", () => {
    expect(isMaskedFor(tag(bruno.id), alice, [alice, { ...bruno, allowsDistribution: false }])).toBe(true);
  });
});

describe("findChild", () => {
  it("résout le visage inconnu sans qu'il soit dans la classe", () => {
    expect(findChild([], UNKNOWN_ID)).toMatchObject({ id: UNKNOWN_ID, allowsDistribution: false });
  });
  it("renvoie undefined pour un élève absent", () => {
    expect(findChild(roster, 42)).toBeUndefined();
  });
});

describe("exportJobs : quelles photos dans quel dossier", () => {
  const photos = [
    { name: "a", tags: [tag(alice.id), tag(bruno.id)] },
    { name: "b", tags: [tag(UNKNOWN_ID)] },
    { name: "c", tags: [tag(alice.id), tag(alice.id)] },
    { name: "d", tags: [] },
  ];
  const jobs = exportJobs(roster, photos);
  const summary = jobs.map((job) => `${job.recipient.name}:${job.photo.name}:${job.index}`);

  it("donne à chaque élève les photos où il a un cadre, numérotées, sans doublon", () => {
    expect(summary).toEqual(["Alice:a:0", "Alice:c:1", "Bruno:a:0"]);
  });
  it("ne crée jamais de dossier pour le visage inconnu ni pour un élève sans photo", () => {
    expect(jobs.some((job) => job.recipient.id === UNKNOWN_ID)).toBe(false);
    expect(jobs.some((job) => job.recipient.id === chloe.id)).toBe(false);
  });
  it("n'exporte pas une photo sans cadre ni un élève absent de la classe", () => {
    expect(jobs.some((job) => job.photo.name === "d" || job.photo.name === "b")).toBe(false);
    expect(exportJobs([], photos)).toEqual([]);
  });
});

describe("forgetTags : élève retiré de la classe", () => {
  it("transforme ses cadres en visages inconnus, sans toucher aux autres", () => {
    const photos = [{ tags: [tag(alice.id), tag(bruno.id)] }, { tags: [tag(chloe.id)] }];
    forgetTags(photos, [alice.id, chloe.id]);
    expect(photos.flatMap((photo) => photo.tags.map((t) => t.childId))).toEqual([UNKNOWN_ID, bruno.id, UNKNOWN_ID]);
  });
});

describe("maskEllipse : zone floutée", () => {
  it("entoure le cadre avec une marge de 1,5 % de chaque côté", () => {
    const mask = maskEllipse({ x: 0.4, y: 0.3, width: 0.2, height: 0.1 }, 1000, 2000)!;
    expect(mask.cx).toBeCloseTo(500);
    expect(mask.cy).toBeCloseTo(700);
    expect(mask.rx).toBeCloseTo((0.1 + MASK_PADDING) * 1000);
    expect(mask.ry).toBeCloseTo((0.05 + MASK_PADDING) * 2000);
    expect(mask.x).toBe(385);
    expect(mask.w).toBe(230);
    expect(mask.y).toBe(570);
    expect(mask.h).toBe(260);
  });
  it("contient toujours tout le cadre", () => {
    const box = { x: 0.123, y: 0.456, width: 0.0777, height: 0.1111 };
    const mask = maskEllipse(box, 4032, 3024)!;
    expect(mask.x).toBeLessThanOrEqual(box.x * 4032);
    expect(mask.y).toBeLessThanOrEqual(box.y * 3024);
    expect(mask.x + mask.w).toBeGreaterThanOrEqual((box.x + box.width) * 4032);
    expect(mask.y + mask.h).toBeGreaterThanOrEqual((box.y + box.height) * 3024);
  });
  it("rogne aux bords sans déplacer l'ellipse", () => {
    const mask = maskEllipse({ x: 0, y: 0, width: 0.2, height: 0.2 }, 1000, 1000)!;
    expect(mask).toMatchObject({ x: 0, y: 0, w: 215, h: 215 });
    expect(mask.cx).toBeCloseTo(100);
    const corner = maskEllipse({ x: 0.9, y: 0.9, width: 0.1, height: 0.1 }, 1000, 1000)!;
    expect(corner.x + corner.w).toBe(1000);
    expect(corner.y + corner.h).toBe(1000);
  });
});

describe("editBox : déplacer / redimensionner un cadre", () => {
  const box = { x: 0.4, y: 0.4, width: 0.2, height: 0.2 };
  it("déplace sans sortir de la photo", () => {
    expect(editBox(box, "move", 0.1, -0.1)).toMatchObject({ width: 0.2, height: 0.2 });
    expect(editBox(box, "move", 1, 1)).toMatchObject({ x: 0.8, y: 0.8 });
    expect(editBox(box, "move", -1, -1)).toMatchObject({ x: 0, y: 0 });
  });
  it("redimensionne par un coin en gardant le coin opposé", () => {
    const next = editBox(box, "se", 0.1, 0.05);
    expect(next.x).toBe(0.4);
    expect(next.width).toBeCloseTo(0.3);
    expect(next.height).toBeCloseTo(0.25);
    const nw = editBox(box, "nw", -0.1, -0.1);
    expect(nw.x + nw.width).toBeCloseTo(0.6);
    expect(nw.y + nw.height).toBeCloseTo(0.6);
  });
  it("ne descend jamais sous la taille minimale ni ne s'inverse", () => {
    const next = editBox(box, "nw", 0.5, 0.5);
    expect(next.width).toBeCloseTo(MIN_TAG_SIZE);
    expect(next.height).toBeCloseTo(MIN_TAG_SIZE);
  });
  it("reste dans la photo", () => {
    const next = editBox(box, "ne", 1, -1);
    expect(next.x + next.width).toBe(1);
    expect(next.y).toBe(0);
  });
});

describe("parseNames : import de la liste des élèves", () => {
  it("lit un nom par ligne, ignore les lignes vides et les doublons", () => {
    expect(parseNames("Alice Martin\r\n\r\n  Bruno   Petit \nAlice Martin\n")).toEqual(["Alice Martin", "Bruno Petit"]);
  });
  it("lit un export tableur avec colonnes Nom et Prénom (prénom d'abord)", () => {
    expect(parseNames("﻿Nom;Prénom;Classe\nMARTIN;Alice;CP\n\"PETIT\";\"Bruno\";CP")).toEqual(["Alice MARTIN", "Bruno PETIT"]);
  });
  it("reconnaît l'en-tête sans accent ni casse, avec tabulations ou virgules", () => {
    expect(parseNames("PRENOM\tNOM\nAlice\tMartin")).toEqual(["Alice Martin"]);
    expect(parseNames("Prénom de l'élève,Nom de famille\nAlice,Martin")).toEqual(["Alice Martin"]);
  });
  it("accepte une seule colonne Prénom ou Nom", () => {
    expect(parseNames("Prénom\nAlice\nBruno")).toEqual(["Alice", "Bruno"]);
    expect(parseNames("Nom;Âge\nMartin;7")).toEqual(["Martin"]);
  });
  it("sans en-tête, prend la première colonne", () => {
    expect(parseNames("Alice;7\nBruno;8")).toEqual(["Alice", "Bruno"]);
  });
  it("renvoie une liste vide pour un fichier vide", () => {
    expect(parseNames("")).toEqual([]);
    expect(parseNames("\n  \n")).toEqual([]);
  });
});

describe("decodeText", () => {
  it("lit l'UTF-8", () => {
    expect(decodeText(new TextEncoder().encode("Chloé Noël"))).toBe("Chloé Noël");
  });
  it("se replie sur Windows-1252 (export tableur Windows)", () => {
    expect(decodeText(new Uint8Array([0x43, 0x68, 0x6c, 0x6f, 0xe9, 0x20, 0x4e, 0x6f, 0xeb, 0x6c]))).toBe("Chloé Noël");
  });
});

describe("noms de fichiers et de dossiers d'export", () => {
  it("remplace les caractères interdits sous Windows dans le nom du dossier", () => {
    expect(cleanFolderName('A/B\\C:D*E?F"G<H>I|J')).toBe("A-B-C-D-E-F-G-H-I-J");
    expect(cleanFolderName("  Chloé  ")).toBe("Chloé");
    expect(cleanFolderName("   ")).toBe("élève");
  });
  it("force l'extension .jpg", () => {
    expect(jpegName("IMG_0001.HEIC")).toBe("IMG_0001.jpg");
    expect(jpegName("sortie.scolaire.png")).toBe("sortie.scolaire.jpg");
    expect(jpegName("sans-extension")).toBe("sans-extension.jpg");
    expect(jpegName(".png")).toBe("photo.jpg");
  });
  it("date le dossier d'export au format JJ-MM-AAAA à HHhMM", () => {
    expect(exportFolderName(new Date(2026, 8, 5, 9, 7))).toBe("Photos du 05-09-2026 à 09h07");
  });
});
