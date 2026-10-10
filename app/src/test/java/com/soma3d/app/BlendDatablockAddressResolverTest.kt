package com.soma3d.app

import java.io.ByteArrayOutputStream
import java.io.File
import java.nio.ByteBuffer
import java.nio.ByteOrder
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class BlendDatablockAddressResolverTest {
    @Test
    fun resolvesOnlyAlignedAddressesWithExpectedSdnaType() {
        val directory = createTempDir(prefix = "blend-address-test")
        val file = File(directory, "sample.blend")
        try {
            file.writeBytes(fileWithPointRecord())
            val block = BlendBlockReader.BlockSummary(
                code = "DATA", payloadBytes = 12, oldAddress = 1000,
                sdnaIndex = 0, count = 1, payloadOffset = 36
            )
            val schema = pointSchema()
            val scan = BlendBlockReader.Result(
                validHeader = true, version = "400", pointerBits = 64,
                littleEndian = true, blocks = listOf(block), schema = schema,
                endedCleanly = true, message = "synthetic fixture"
            )
            val source = BlendCachedFileSource.open(file, 64, true)!!
            val decoder = BlendDatablockDecoder(scan, source)

            val resolved = decoder.decodeRecordsAtAddress(1000, "Point", 1)
            assertTrue(resolved.message, resolved.success)
            assertEquals(1, resolved.records.size)
            assertEquals(2.5, resolved.records.single().fields["x"] as Double, 0.00001)

            assertFalse(decoder.decodeRecordsAtAddress(1001, "Point", 1).success)
            assertFalse(decoder.decodeRecordsAtAddress(1000, "Other", 1).success)
        } finally {
            directory.deleteRecursively()
        }
    }

    private fun pointSchema() = BlendBlockReader.Schema(
        names = listOf("x", "y", "z"),
        types = listOf("char", "float", "Point"),
        typeLengths = listOf(1, 4, 12),
        structs = listOf(
            BlendBlockReader.SchemaStruct(
                "Point",
                listOf(
                    BlendBlockReader.SchemaField("float", "x"),
                    BlendBlockReader.SchemaField("float", "y"),
                    BlendBlockReader.SchemaField("float", "z")
                )
            )
        )
    )

    private fun fileWithPointRecord(): ByteArray {
        val out = ByteArrayOutputStream()
        out.write("BLENDER_v400".toByteArray(Charsets.US_ASCII))
        out.write("DATA".toByteArray(Charsets.US_ASCII))
        writeInt(out, 12)
        writeLong(out, 1000L)
        writeInt(out, 0)
        writeInt(out, 1)
        out.write(ByteBuffer.allocate(12).order(ByteOrder.LITTLE_ENDIAN)
            .putFloat(2.5f).putFloat(-1.0f).putFloat(9.0f).array())
        return out.toByteArray()
    }

    private fun writeInt(out: ByteArrayOutputStream, value: Int) {
        out.write(value and 0xff)
        out.write((value ushr 8) and 0xff)
        out.write((value ushr 16) and 0xff)
        out.write((value ushr 24) and 0xff)
    }

    private fun writeLong(out: ByteArrayOutputStream, value: Long) {
        repeat(8) { index -> out.write(((value ushr (8 * index)) and 0xff).toInt()) }
    }
}
