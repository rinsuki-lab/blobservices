import { expect, test } from "@playwright/test"

const URLS = {
    blobgateway: "http://localhost:3003",
    blobmanager: "http://localhost:3001",
}

const TEST_PREFIX = `blobservices_test_${crypto.randomUUID()}`

test("simple upload & download", async ({request}) => {
    const data = "hello, world!"

    const uploadRes = await request.put(`${URLS.blobgateway}/v1/content/by-ref/${TEST_PREFIX}/simple`, {
        data: "hello, world!",
        failOnStatusCode: true,
    })

    const downloadRes = await request.get(`${URLS.blobgateway}/v1/content/by-ref/${TEST_PREFIX}/simple`, {
        failOnStatusCode: true,
    })
    expect(await downloadRes.text()).toStrictEqual(data)
})
