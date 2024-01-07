#ifndef ACB_ACB_H_
#define ACB_ACB_H_

#include "acb/src/lib.rs.h"
#include "rust/cxx.h"

#include <cstdint>
#include <vector>

constexpr uint64_t mltd_hca_key  = 765765765765765ULL;
constexpr uint32_t mltd_hca_key1 = mltd_hca_key & 0xffffffff;
constexpr uint32_t mltd_hca_key2 = mltd_hca_key >> 32;

rust::Vec<Track> to_tracks(rust::Slice<const std::uint8_t> buf);

#endif // ACB_ACB_H_
