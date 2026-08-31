//! Chunking rules, all source kinds (docs/04 §10). Pure functions — tested hard
//! because they are the cheapest place to be correct (docs/09 §2).

use std::sync::OnceLock;
use tiktoken_rs::CoreBPE;

pub const TARGET_TOKENS: usize = 800;
pub const MAX_TOKENS: usize = 1200;
pub const MIN_TOKENS: usize = 80;
pub const OVERLAP_RATIO: f32 = 0.15;
pub const OVERLAP_MAX_TOKENS: usize = 120;

fn bpe() -> &'static CoreBPE {
    static BPE: OnceLock<CoreBPE> = OnceLock::new();
    BPE.get_or_init(|| tiktoken_rs::cl100k_base().expect("cl100k_base"))
}

/// Approximate token count (docs/04 §10 — exact match not required).
pub fn count_tokens(text: &str) -> usize {
    bpe().encode_ordinary(text).len()
}

#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    pub text: String,
    pub tokens: usize,
    /// Char offset into the original text (for line/char locators).
    pub start: usize,
    pub end: usize,
}

/// Split `text` into chunks. Boundaries are tried in priority order:
/// heading → blank-line (paragraph) → sentence end → hard character count.
pub fn split(text: &str) -> Vec<Chunk> {
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }

    // 1. Segment into atoms at the strongest available boundary.
    let atoms = segment(text);

    // 2. Greedily pack atoms up to TARGET, never past MAX.
    let mut chunks: Vec<Chunk> = Vec::new();
    let mut cur = String::new();
    let mut cur_start = atoms.first().map(|a| a.start).unwrap_or(0);
    let mut cur_end = cur_start;

    for atom in &atoms {
        let atom_tokens = count_tokens(&atom.text);
        let cur_tokens = count_tokens(&cur);

        if atom_tokens > MAX_TOKENS {
            // Atom itself too big — flush current, then hard-split the atom.
            if !cur.trim().is_empty() {
                push_chunk(&mut chunks, &cur, cur_start, cur_end);
                cur.clear();
            }
            for piece in hard_split(&atom.text, atom.start) {
                chunks.push(Chunk {
                    tokens: count_tokens(&piece.text),
                    ..piece
                });
            }
            cur_start = atom.end;
            cur_end = atom.end;
            continue;
        }

        if cur_tokens + atom_tokens > TARGET_TOKENS && cur_tokens >= MIN_TOKENS {
            push_chunk(&mut chunks, &cur, cur_start, cur_end);
            // Overlap: carry the tail of the previous chunk.
            let tail = overlap_tail(&cur);
            cur = if tail.is_empty() { String::new() } else { format!("{tail}\n") };
            cur_start = atom.start.saturating_sub(tail.len());
        }
        if cur.is_empty() {
            cur_start = atom.start;
        }
        if !cur.is_empty() && !cur.ends_with('\n') {
            cur.push('\n');
        }
        cur.push_str(&atom.text);
        cur_end = atom.end;
    }

    if !cur.trim().is_empty() {
        // Merge a too-small tail into the previous chunk (no heading-only chunks).
        if count_tokens(&cur) < MIN_TOKENS {
            if let Some(last) = chunks.last_mut() {
                last.text.push('\n');
                last.text.push_str(cur.trim());
                last.tokens = count_tokens(&last.text);
                last.end = cur_end;
            } else {
                push_chunk(&mut chunks, &cur, cur_start, cur_end);
            }
        } else {
            push_chunk(&mut chunks, &cur, cur_start, cur_end);
        }
    }

    chunks
}

fn push_chunk(chunks: &mut Vec<Chunk>, text: &str, start: usize, end: usize) {
    let t = text.trim();
    if t.is_empty() {
        return;
    }
    chunks.push(Chunk {
        text: t.to_string(),
        tokens: count_tokens(t),
        start,
        end,
    });
}

struct Atom {
    text: String,
    start: usize,
    end: usize,
}

/// Split into paragraph atoms (blank line), further split any paragraph that is
/// still over MAX into sentences.
fn segment(text: &str) -> Vec<Atom> {
    let mut atoms = Vec::new();
    let mut offset = 0usize;
    for para in text.split("\n\n") {
        let para_start = text[offset..]
            .find(para)
            .map(|i| offset + i)
            .unwrap_or(offset);
        offset = para_start + para.len();
        let para = para.trim();
        if para.is_empty() {
            continue;
        }
        if count_tokens(para) <= MAX_TOKENS {
            atoms.push(Atom {
                text: para.to_string(),
                start: para_start,
                end: para_start + para.len(),
            });
        } else {
            let mut s = para_start;
            for sent in split_sentences(para) {
                atoms.push(Atom {
                    text: sent.to_string(),
                    start: s,
                    end: s + sent.len(),
                });
                s += sent.len();
            }
        }
    }
    atoms
}

/// Sentence boundary on `。．.!?！？` optionally followed by a closing bracket.
fn split_sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        cur.push(c);
        if matches!(c, '。' | '．' | '.' | '!' | '?' | '！' | '？') {
            while let Some(&n) = chars.peek() {
                if matches!(n, '」' | '』' | ')' | '）' | '"' | '\'') {
                    cur.push(n);
                    chars.next();
                } else {
                    break;
                }
            }
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}

/// Last-resort: cut by character budget matching ~TARGET tokens.
fn hard_split(text: &str, base_start: usize) -> Vec<Chunk> {
    // ~4 chars/token for latin, ~1.5 for CJK; use 2.5 as a middle estimate.
    let budget_chars = TARGET_TOKENS * 3;
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut byte_off = base_start;
    while i < chars.len() {
        let end = (i + budget_chars).min(chars.len());
        let piece: String = chars[i..end].iter().collect();
        let plen = piece.len();
        out.push(Chunk {
            tokens: 0,
            text: piece.trim().to_string(),
            start: byte_off,
            end: byte_off + plen,
        });
        byte_off += plen;
        i = end;
    }
    out
}

fn overlap_tail(chunk: &str) -> String {
    let target = (count_tokens(chunk) as f32 * OVERLAP_RATIO) as usize;
    let target = target.min(OVERLAP_MAX_TOKENS);
    if target == 0 {
        return String::new();
    }
    // Take whole trailing sentences until we reach ~target tokens.
    let sentences = split_sentences(chunk);
    let mut tail = String::new();
    for s in sentences.iter().rev() {
        if count_tokens(&tail) >= target {
            break;
        }
        tail = format!("{s}{tail}");
    }
    tail.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_yields_nothing() {
        assert!(split("").is_empty());
        assert!(split("   \n  ").is_empty());
    }

    #[test]
    fn short_text_is_one_chunk() {
        let c = split("Just a short line of text.");
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].text, "Just a short line of text.");
    }

    #[test]
    fn long_english_splits_and_overlaps() {
        let para = "Sentence number one is here. ".repeat(400); // well over target
        let chunks = split(&para);
        assert!(chunks.len() > 1, "expected multiple chunks, got {}", chunks.len());
        for c in &chunks {
            assert!(c.tokens <= MAX_TOKENS, "chunk over MAX: {}", c.tokens);
        }
        // consecutive chunks share a tail (overlap)
        let a_tail: String = chunks[0].text.chars().rev().take(30).collect();
        let a_tail: String = a_tail.chars().rev().collect();
        assert!(chunks[1].text.contains(a_tail.trim_end_matches(|c: char| !c.is_alphanumeric()).trim()));
    }

    #[test]
    fn japanese_splits_on_ideographic_period() {
        let para = "これはテスト文です。".repeat(400);
        let chunks = split(&para);
        assert!(chunks.len() > 1);
        for c in &chunks {
            assert!(c.tokens <= MAX_TOKENS);
        }
    }

    #[test]
    fn tiny_trailing_block_merges_back() {
        let big = "word ".repeat(900);
        let text = format!("{big}\n\n# tiny");
        let chunks = split(&text);
        // "# tiny" is under MIN — it must not be its own chunk.
        assert!(!chunks.last().unwrap().text.trim().eq("# tiny"));
        assert!(chunks.last().unwrap().text.contains("# tiny"));
    }
}
