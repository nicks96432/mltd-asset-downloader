#include <cstddef>
#include <iostream>
#include <string>
#include <unordered_set>
#include <vector>

#include "ichinose/CAcbFile.h"
#include "ichinose/CAcbHelper.h"
#include "ichinose/CAfs2Archive.h"
#include "kawashima/hca/CDefaultWaveGenerator.h"
#include "kawashima/hca/CHcaCipherConfig.h"
#include "kawashima/hca/CHcaDecoder.h"
#include "kawashima/hca/CHcaDecoderConfig.h"
#include "kawashima/hca/CHcaFormatReader.h"
#include "takamori/exceptions/CException.h"
#include "takamori/streams/CMemoryStream.h"

#include "./acb.h"

static auto decode_stream(acb::CMemoryStream *stream, const acb::CHcaDecoderConfig &config)
    -> rust::Vec<std::uint8_t> {
    acb::CHcaDecoder decoder = {stream, config};

    std::array<uint8_t, 4096> buf;
    std::size_t read = 1;

    rust::Vec<std::uint8_t> decoded_data;
    decoded_data.reserve(stream->GetLength());

    do {
        read = decoder.Read(buf.data(), buf.size(), 0, buf.size());
        std::copy(buf.cbegin(), buf.cbegin() + read, std::back_inserter(decoded_data));
    } while (read > 0);

    return decoded_data;
}

auto to_tracks(rust::Slice<const std::uint8_t> buf) -> rust::Vec<Track> {
    std::vector<std::uint8_t> buf_clone = {buf.begin(), buf.end()};
    acb::CMemoryStream stream           = {
        buf_clone.data(), static_cast<std::uint64_t>(buf_clone.size()), false
    };
    acb::CAcbFile acb_file = {&stream, ""};
    acb_file.Initialize();

    const acb::CAfs2Archive *archive;
#ifdef DEBUG
    const std::uint32_t format_version = acb_file.GetFormatVersion();
    std::cerr << "format_version:" << format_version << '\n';
#endif

    try {
        archive = acb_file.GetInternalAwb();
    } catch (acb::CException &e) {
        std::cerr << e.GetExceptionMessage() << " (" << e.GetOpResult() << ")\n";
        archive = nullptr;
    }

    rust::Vec<Track> tracks;

    if (archive == nullptr)
        return tracks;

    std::uint16_t key_modifer = 0;
    if (acb_file.GetFormatVersion() >= acb::CAcbFile::KEY_MODIFIER_ENABLED_VERSION)
        key_modifer = archive->GetHcaKeyModifier();

    acb::CHcaDecoderConfig decoder_config{};
    decoder_config.waveHeaderEnabled = TRUE;
    decoder_config.decodeFunc        = acb::CDefaultWaveGenerator::Decode16BitS;
    decoder_config.cipherConfig = acb::CHcaCipherConfig{mltd_hca_key1, mltd_hca_key2, key_modifer};

    std::unordered_set<std::uint32_t> extracted_cue_ids;

    // extract files with readable cue names
    for (const std::string &filename : acb_file.GetFileNames()) {
        if (filename.empty())
            continue;

        acb::CMemoryStream *entry_data_stream =
            dynamic_cast<acb::CMemoryStream *>(acb_file.OpenDataStream(filename.c_str()));
        if (entry_data_stream == nullptr) {
#ifdef DEBUG
            std::cerr << "cue of '" << filename.c_str() << "' cannot be retrived\n";
#endif
            continue;
        }

        const AFS2_FILE_RECORD *file_record =
            acb_file.GetFileRecordByWaveformFileName(filename.c_str());
        assert(file_record != nullptr);

        if (!acb::CHcaFormatReader::IsPossibleHcaStream(entry_data_stream)) {
#ifdef DEBUG
            std::cerr << "not HCA, skipping\n";
#endif
            continue;
        }

        tracks.push_back(Track{filename, decode_stream(entry_data_stream, decoder_config)});
        extracted_cue_ids.insert(file_record->cueId);
        delete entry_data_stream;
    }

    // extract files that are not yet exported
    for (const auto &entry : archive->GetFiles()) {
        const AFS2_FILE_RECORD &record = entry.second;

        if (extracted_cue_ids.find(record.cueId) != extracted_cue_ids.end())
            continue;

        const std::string &filename = acb_file.GetCueNameByCueId(record.cueId);

        acb::CMemoryStream *entry_data_stream = acb::CAcbHelper::ExtractToNewStream(
            acb_file.GetStream(),
            record.fileOffsetAligned,
            static_cast<std::uint32_t>(record.fileSize)
        );

        tracks.push_back(Track{filename, decode_stream(entry_data_stream, decoder_config)});
        delete entry_data_stream;
    }

    return tracks;
}
