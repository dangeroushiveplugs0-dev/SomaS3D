package com.soma3d.app

/**
 * Connects block metadata, cached payload reads, SDNA decoding, and file-address lookup.
 *
 * This stage produces conservative generic records; it does not yet interpret Blender
 * Object/Mesh semantics or execute stored driver/script content.
 */
class BlendDatablockDecoder(
    private val scan: BlendBlockReader.Result,
    private val source: BlendCachedFileSource,
    private val maxPayloadBytes: Int = 16 * 1024 * 1024,
    private val maxRecordsPerBlock: Int = 100_000
) {
    data class DecodedBlock(
        val block: BlendBlockReader.BlockSummary,
        val typeName: String?,
        val records: List<BlendStructDecoder.DecodedRecord>,
        val success: Boolean,
        val message: String
    )

    data class DecodeSummary(
        val blocks: List<DecodedBlock>,
        val decodedBlockCount: Int,
        val skippedBlockCount: Int,
        val failedBlockCount: Int,
        val message: String
    )

    private val addressIndex = BlendBlockIndex.create(scan.blocks)

    fun decodeBlock(block: BlendBlockReader.BlockSummary): DecodedBlock {
        if (!scan.validHeader || scan.schema == null || scan.pointerBits == null || scan.littleEndian == null) {
            return failure(block, null, "A valid header and SDNA schema are required.")
        }
        if (block.code == "DNA1" || block.code == "ENDB") {
            return failure(block, null, "Structural block is not a datablock record.")
        }
        val schema = scan.schema
        if (block.sdnaIndex < 0L || block.sdnaIndex >= schema.structs.size.toLong()) {
            return failure(block, null, "Block SDNA index is outside the schema.")
        }
        if (block.count < 0L || block.count > maxRecordsPerBlock.toLong()) {
            return failure(block, schema.structs[block.sdnaIndex.toInt()].typeName, "Block record count exceeds the safe decode limit.")
        }
        val payloadResult = source.readPayload(block, maxPayloadBytes)
        val payload = payloadResult.payload
            ?: return failure(block, schema.structs[block.sdnaIndex.toInt()].typeName, payloadResult.message)
        val decoded = BlendStructDecoder.decodeRecords(
            schema = schema,
            sdnaIndex = block.sdnaIndex,
            count = block.count,
            payload = payload,
            pointerBits = scan.pointerBits,
            littleEndian = scan.littleEndian,
            maxRecords = maxRecordsPerBlock
        )
        val typeName = schema.structs[block.sdnaIndex.toInt()].typeName
        val layout = BlendStructDecoder.layout(schema, typeName, scan.pointerBits)
        val addressed = if (decoded.success && layout.supported) decoded.records.mapIndexed { index, record ->
            record.copy(fileAddress = recordAddress(block.oldAddress, index, layout.byteSize))
        } else decoded.records
        return DecodedBlock(block, typeName, addressed, decoded.success, decoded.message)
    }

    /** Decodes only blocks whose SDNA types match the requested names. */
    fun decodeTypes(typeNames: Set<String>, maxBlocks: Int = 20_000): DecodeSummary {
        val schema = scan.schema
            ?: return DecodeSummary(emptyList(), 0, 0, 0, "SDNA schema is unavailable.")
        val candidates = scan.blocks.filter { block ->
            block.code != "DNA1" && block.code != "ENDB" &&
                block.sdnaIndex >= 0L && block.sdnaIndex < schema.structs.size.toLong() &&
                schema.structs[block.sdnaIndex.toInt()].typeName in typeNames
        }
        val selected = candidates.take(maxBlocks.coerceAtLeast(0))
        val results = selected.map { decodeBlock(it) }
        val successful = results.count { it.success }
        val failed = results.count { !it.success }
        val skipped = candidates.size - selected.size
        return DecodeSummary(
            results, successful, skipped, failed,
            "Decoded $successful requested-type block(s); $failed failed or unsupported; $skipped not attempted."
        )
    }

    /** Decodes blocks conservatively; malformed or unsupported blocks are reported individually. */
    fun decodeAll(maxBlocks: Int = 20_000): DecodeSummary {
        val candidates = scan.blocks.filter { it.code != "DNA1" && it.code != "ENDB" }
        val selected = candidates.take(maxBlocks.coerceAtLeast(0))
        val results = selected.map { decodeBlock(it) }
        val successful = results.count { it.success }
        val failed = results.count { !it.success }
        val skipped = (candidates.size - selected.size) + results.count {
            it.message == "Structural block is not a datablock record."
        }
        return DecodeSummary(
            results, successful, skipped, failed,
            "Decoded $successful block(s); $failed failed or unsupported; $skipped not attempted."
        )
    }


    data class AddressDecodedRecords(
        val block: BlendBlockReader.BlockSummary?,
        val firstRecordIndex: Long,
        val records: List<BlendStructDecoder.DecodedRecord>,
        val success: Boolean,
        val message: String
    )

    /** Resolves a stored file address to a bounded run of records of an expected SDNA type. */
    fun decodeRecordsAtAddress(
        fileAddress: Long,
        expectedType: String,
        recordCount: Int
    ): AddressDecodedRecords {
        if (fileAddress <= 0L || expectedType.isBlank() ||
            recordCount < 0 || recordCount > maxRecordsPerBlock) {
            return addressFailure(null, 0L, "Address or record count is outside safe limits.")
        }
        if (recordCount == 0) {
            return AddressDecodedRecords(null, 0L, emptyList(), true, "No records requested.")
        }
        val schema = scan.schema
            ?: return addressFailure(null, 0L, "SDNA schema is unavailable.")
        val entry = addressIndex.index.findContaining(fileAddress)
            ?: return addressFailure(null, 0L, "Stored address is not inside an indexed datablock.")
        val block = entry.block
        if (block.sdnaIndex < 0L || block.sdnaIndex >= schema.structs.size.toLong()) {
            return addressFailure(block, 0L, "Target block has an invalid SDNA index.")
        }
        val actualType = schema.structs[block.sdnaIndex.toInt()].typeName
        if (actualType != expectedType) {
            return addressFailure(block, 0L, "Expected $expectedType records, but address resolves to $actualType.")
        }
        val layout = BlendStructDecoder.layout(schema, actualType, scan.pointerBits ?: 0)
        if (!layout.supported || layout.byteSize <= 0) {
            return addressFailure(block, 0L, "Target record layout is unsupported: ${layout.message}")
        }
        val byteOffset = fileAddress - entry.startAddress
        if (byteOffset % layout.byteSize.toLong() != 0L) {
            return addressFailure(block, 0L, "Stored address is not aligned to a $actualType record.")
        }
        val firstRecord = byteOffset / layout.byteSize.toLong()
        val requestedEnd = try {
            Math.addExact(firstRecord, recordCount.toLong())
        } catch (_: ArithmeticException) {
            return addressFailure(block, firstRecord, "Requested record range overflowed.")
        }
        if (firstRecord < 0L || requestedEnd > block.count) {
            return addressFailure(block, firstRecord, "Requested records exceed the target block's declared count.")
        }
        val byteCount = try {
            Math.multiplyExact(recordCount.toLong(), layout.byteSize.toLong())
        } catch (_: ArithmeticException) {
            return addressFailure(block, firstRecord, "Requested byte range overflowed.")
        }
        if (byteOffset > block.payloadBytes || byteCount > block.payloadBytes - byteOffset ||
            byteOffset > Int.MAX_VALUE || byteCount > Int.MAX_VALUE) {
            return addressFailure(block, firstRecord, "Requested records are outside the target payload.")
        }
        val payloadResult = source.readPayload(block, maxPayloadBytes)
        val payload = payloadResult.payload
            ?: return addressFailure(block, firstRecord, payloadResult.message)
        val start = byteOffset.toInt()
        val end = start.toLong() + byteCount
        if (end > payload.size.toLong()) {
            return addressFailure(block, firstRecord, "Requested records exceed the retrieved payload.")
        }
        val decoded = BlendStructDecoder.decodeRecords(
            schema = schema,
            sdnaIndex = block.sdnaIndex,
            count = recordCount.toLong(),
            payload = payload.copyOfRange(start, end.toInt()),
            pointerBits = scan.pointerBits ?: 0,
            littleEndian = scan.littleEndian ?: true,
            maxRecords = maxRecordsPerBlock
        )
        val addressed = if (decoded.success) decoded.records.mapIndexed { index, record ->
            record.copy(fileAddress = recordAddress(fileAddress, index, layout.byteSize))
        } else decoded.records
        return AddressDecodedRecords(block, firstRecord, addressed, decoded.success, decoded.message)
    }

    private fun recordAddress(base: Long, index: Int, recordSize: Int): Long? = try {
        Math.addExact(base, Math.multiplyExact(index.toLong(), recordSize.toLong()))
    } catch (_: ArithmeticException) {
        null
    }

    private fun addressFailure(
        block: BlendBlockReader.BlockSummary?,
        firstRecord: Long,
        message: String
    ) = AddressDecodedRecords(block, firstRecord, emptyList(), false, message)

    fun findBlockContaining(fileAddress: Long): BlendBlockReader.BlockSummary? =
        addressIndex.index.findContaining(fileAddress)?.block

    private fun failure(
        block: BlendBlockReader.BlockSummary,
        typeName: String?,
        message: String
    ) = DecodedBlock(block, typeName, emptyList(), false, message)
}
