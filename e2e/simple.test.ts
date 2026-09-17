import { expect, test } from "@playwright/test"
import { create, toJson } from "@bufbuild/protobuf"
import { PutBlobRefRequestSchema } from "./gen/manager_pb"

const URLS = {
    blobgateway: "http://localhost:3003",
    blobmanager: "http://localhost:3001",
}

const namespace = `blobservices_test_${crypto.randomUUID()}`

test("simple upload & download", async ({request}) => {
    const data = "hello, world!"
    const key = "simple"

    await request.put(`${URLS.blobgateway}/v1/content/by-ref/${namespace}/${key}`, {
        data,
        failOnStatusCode: true,
    })

    const downloadRes = await request.get(`${URLS.blobgateway}/v1/content/by-ref/${namespace}/${key}`, {
        failOnStatusCode: true,
    })
    expect(await downloadRes.text()).toStrictEqual(data)
})

test("from other ref", async ({request}) => {
    const data = "hello, world!"
    const key1 = "otherref-parent"
    const key2 = "otherref-child"

    await request.put(`${URLS.blobgateway}/v1/content/by-ref/${namespace}/${key1}`, {
        data,
        failOnStatusCode: true,
    })

    await request.put(`${URLS.blobmanager}/v1/refs/${namespace}/${key2}`, {
        data: toJson(PutBlobRefRequestSchema, create(PutBlobRefRequestSchema, {
            content: {
                case: "fromOtherRef",
                value: {
                    namespace: namespace,
                    key: key1,
                }
            }
        })),
        failOnStatusCode: true,
    })

    const downloadRes = await request.get(`${URLS.blobgateway}/v1/content/by-ref/${namespace}/${key2}`, {
        failOnStatusCode: true,
    })
    expect(await downloadRes.text()).toStrictEqual(data)
})

test("slice", async ({request}) => {
    const data = "hello, world!"
    const key1 = "slice-parent"
    const key2 = "slice-child"

    await request.put(`${URLS.blobgateway}/v1/content/by-ref/${namespace}/${key1}`, {
        data,
        failOnStatusCode: true,
    })

    await request.put(`${URLS.blobmanager}/v1/refs/${namespace}/${key2}`, {
        data: toJson(PutBlobRefRequestSchema, create(PutBlobRefRequestSchema, {
            content: {
                case: "fromBlobSlice",
                value: {
                    source: {
                        content: {
                            case: "fromOtherRef",
                            value: {
                                namespace: namespace,
                                key: key1,
                            },
                        },
                    },
                    start: 2n,
                    size: 3n,
                    hashes: {},
                }
            }
        })),
        failOnStatusCode: true,
    })

    const downloadRes = await request.get(`${URLS.blobgateway}/v1/content/by-ref/${namespace}/${key2}`, {
        failOnStatusCode: true,
    })
    expect(await downloadRes.text()).toStrictEqual(data.slice(2, 5))
})

test("nested slice", async ({request}) => {
    let data = "hello, world!"
    let key = "nested-slice-parent"

    await request.put(`${URLS.blobgateway}/v1/content/by-ref/${namespace}/${key}`, {
        data,
        failOnStatusCode: true,
    })

    const slices = [[2, 9], [3, 5], [1, 2]] as const
    for (const [index, [start, size]] of slices.entries()) {
        const nextKey = `nested-slice-${index}`
        await request.put(`${URLS.blobmanager}/v1/refs/${namespace}/${nextKey}`, {
            data: toJson(PutBlobRefRequestSchema, create(PutBlobRefRequestSchema, {
                content: {
                    case: "fromBlobSlice",
                    value: {
                        source: {
                            content: {
                                case: "fromOtherRef",
                                value: { namespace, key },
                            },
                        },
                        start: BigInt(start),
                        size: BigInt(size),
                        hashes: {},
                    },
                },
            })),
            failOnStatusCode: true,
        })
        key = nextKey
        data = data.slice(start, start + size)
    }

    const downloadRes = await request.get(`${URLS.blobgateway}/v1/content/by-ref/${namespace}/${key}`, {
        failOnStatusCode: true,
    })
    expect(downloadRes.status()).toBe(200)
    expect(downloadRes.headers()["content-length"]).toBe(String(data.length))
    expect(await downloadRes.text()).toStrictEqual(data)
})
