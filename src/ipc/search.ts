import { call } from "./client";
import type { SearchQuery, SearchResults } from "./types.gen";

export const searchApi = {
  query: (q: SearchQuery) => call<SearchResults>("search_query", { query: q }),
};
