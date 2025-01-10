#ifndef ACB_ACB_H_
#define ACB_ACB_H_

#include <cstdint>
#include <vector>

#include "acb/src/lib.rs.h"
#include "rust/cxx.h"

constexpr std::uint64_t mltd_hca_key  = 765765765765765ULL;
constexpr std::uint32_t mltd_hca_key1 = mltd_hca_key & 0xffffffff;
constexpr std::uint32_t mltd_hca_key2 = mltd_hca_key >> 32;

auto to_tracks(rust::Slice<const std::uint8_t> buf) -> rust::Vec<Track>;

#endif // ACB_ACB_H_
