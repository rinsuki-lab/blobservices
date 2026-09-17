import { expect, test } from "@playwright/test"
import { namespace, URLS } from "./shared"
import { create, toJson } from "@bufbuild/protobuf"
import { PutBlobRefRequestSchema } from "./gen/manager_pb"

const decryptedShortData = "hello, world!"
const decryptedLongData = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum."

const aesKey128 = new TextEncoder().encode("aes128examplekey")
const aesKey192 = new TextEncoder().encode("aes-192-cbc__example-key")
const iv = new TextEncoder().encode("aescbc-exampleiv")

test("simple-aes128-short", async ({request}) => {
    const keyEncrypted = "simple-aes128-enc"
    const keyDecrypted = "simple-aes128-dec"
    const rawData = new TextEncoder().encode(decryptedShortData)
    const encryptedData = await crypto.subtle.encrypt(
        {
            name: "AES-CBC",
            iv,
        },
        await crypto.subtle.importKey("raw", aesKey128, "AES-CBC", false, ["encrypt"]),
        rawData
    )

    await request.put(`${URLS.blobgateway}/v1/content/by-ref/${namespace}/${keyEncrypted}`, {
        data: encryptedData,
        failOnStatusCode: true,
    })

    await request.put(`${URLS.blobmanager}/v1/refs/${namespace}/${keyDecrypted}`, {
        data: toJson(PutBlobRefRequestSchema, create(PutBlobRefRequestSchema, {
            content: {
                case: "fromBlobTransform",
                value: {
                    source: {
                        content: {
                            case: "fromOtherRef",
                            value: {
                                namespace,
                                key: keyEncrypted,
                            }
                        },
                    },
                    size: BigInt(rawData.byteLength),
                    hashes: {},
                    transform: {
                        params: {
                            transform: {
                                case: "decAesCbc",
                                value: {
                                    key: aesKey128,
                                    iv,
                                    ivPrepended: false,
                                    padding: {
                                        padding: {
                                            case: "pkcs7",
                                            value: {},
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        })),
        failOnStatusCode: true,
    })

    const downloadRes = await request.get(`${URLS.blobgateway}/v1/content/by-ref/${namespace}/${keyDecrypted}`, {
        failOnStatusCode: true,
    })
    expect(await downloadRes.text()).toStrictEqual(decryptedShortData)
})
