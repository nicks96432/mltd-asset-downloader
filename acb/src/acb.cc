#include "acb.h"

#include <iostream>
#include <stdexcept>
#include <string>
#include <unordered_set>
#include <vector>

#include "cgss_enum.h"
#include "ichinose/CAcbFile.h"
#include "ichinose/CAcbHelper.h"
#include "ichinose/CAfs2Archive.h"
#include "kawashima/hca/CDefaultWaveGenerator.h"
#include "kawashima/hca/CHcaDecoder.h"
#include "kawashima/hca/CHcaFormatReader.h"
#include "takamori/exceptions/CException.h"
#include "takamori/streams/CMemoryStream.h"

static rust::Vec<std::uint8_t>
decode_stream(cgss::CMemoryStream *stream, const cgss::CHcaDecoderConfig &config) {
    cgss::CHcaDecoder decoder{stream, config};

    std::array<uint8_t, 4096> buf{};
    std::uint32_t read = 1;

    rust::Vec<std::uint8_t> decoded_data{};
    decoded_data.reserve(stream->GetLength());

    do {
        read = decoder.Read(buf.data(), buf.size(), 0, buf.size());
        for (auto it = buf.cbegin(); it != buf.cbegin() + read; ++it)
            decoded_data.push_back(*it);
    } while (read > 0);

    return decoded_data;
}

rust::Vec<Track> to_wav(const rust::Vec<std::uint8_t> &buf) {
    std::vector<std::uint8_t> buf_clone = {buf.cbegin(), buf.cend()};
    cgss::CMemoryStream stream{buf_clone.data(), static_cast<std::uint64_t>(buf.size()), false};
    cgss::CAcbFile acb{&stream, ""};
    acb.Initialize();

    const cgss::CAfs2Archive *archive;
#ifdef DEBUG
    const std::uint32_t format_version = acb.GetFormatVersion();
    std::cerr << "format_version:" << format_version << '\n';
#endif

    try {
        archive = acb.GetInternalAwb();
    } catch (cgss::CException &e) {
        std::cerr << e.GetExceptionMessage() << " (" << e.GetOpResult() << ")\n";
        archive = nullptr;
    }

    rust::Vec<Track> tracks{};

    if (archive == nullptr)
        return tracks;

    uint16_t key_modifer = 0;
    if (acb.GetFormatVersion() >= cgss::CAcbFile::KEY_MODIFIER_ENABLED_VERSION)
        key_modifer = archive->GetHcaKeyModifier();

    cgss::CHcaDecoderConfig decoder_config{};
    decoder_config.waveHeaderEnabled = TRUE;
    decoder_config.decodeFunc        = cgss::CDefaultWaveGenerator::Decode16BitS;
    decoder_config.cipherConfig      = cgss::CHcaCipherConfig{mltd_hca_key1, mltd_hca_key2, key_modifer};

    std::unordered_set<uint32_t> extracted_cue_ids;

    // extract files with readable cue names
    for (const std::string &filename : acb.GetFileNames()) {
        if (filename.empty())
            continue;

        cgss::CMemoryStream *entry_data_stream =
            dynamic_cast<cgss::CMemoryStream *>(acb.OpenDataStream(filename.c_str()));
        if (entry_data_stream == nullptr) {
#ifdef DEBUG
            std::cerr << "cue of '" << filename.c_str() << "' cannot be retrived\n";
#endif
            continue;
        }

        const AFS2_FILE_RECORD *file_record = acb.GetFileRecordByWaveformFileName(filename.c_str());
        assert(file_record != nullptr);

        if (!cgss::CHcaFormatReader::IsPossibleHcaStream(entry_data_stream)) {
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

        const std::string &filename = acb.GetCueNameByCueId(record.cueId);

        cgss::CMemoryStream *entry_data_stream = cgss::CAcbHelper::ExtractToNewStream(
            acb.GetStream(), record.fileOffsetAligned, static_cast<std::uint32_t>(record.fileSize)
        );

        tracks.push_back(Track{filename, decode_stream(entry_data_stream, decoder_config)});
        delete entry_data_stream;
    }

    return tracks;
}
