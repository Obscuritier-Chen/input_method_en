import json
import struct


INPUT_PATH = (
    "model/datasets/candidate/"
    "prefix_candidates.json"
)

OUTPUT_PATH = (
    "model/datasets/candidate/"
    "candidate_index.bin"
)


def main():

    print("Loading json...")

    with open(
        INPUT_PATH,
        "r",
        encoding="utf8"
    ) as f:

        table = json.load(f)


    print(
        f"Prefixes: {len(table)}"
    )


    with open(
        OUTPUT_PATH,
        "wb"
    ) as f:


        ################################################
        # Header
        ################################################

        # prefix数量
        f.write(
            struct.pack(
                "<I",
                len(table)
            )
        )


        ################################################
        # Body
        ################################################

        for prefix, candidates in table.items():


            # ----------------------------
            # prefix
            # ----------------------------

            prefix_bytes = prefix.encode(
                "utf8"
            )


            # prefix长度
            f.write(
                struct.pack(
                    "<H",
                    len(prefix_bytes)
                )
            )


            # prefix内容
            f.write(
                prefix_bytes
            )


            # ----------------------------
            # candidate ids
            # ----------------------------

            # candidate数量
            f.write(
                struct.pack(
                    "<H",
                    len(candidates)
                )
            )


            for cid in candidates:

                f.write(
                    struct.pack(
                        "<i",
                        cid
                    )
                )


    print(
        "Build finished."
    )



if __name__ == "__main__":

    main()