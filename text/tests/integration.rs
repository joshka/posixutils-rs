//
// Copyright (c) 2024 Jeff Garzik
// Copyright (c) 2024 Hemi Labs, Inc.
//
// This file is part of the posixutils-rs project covered under
// the MIT License.  For the full license text, please see the LICENSE
// file in the root directory of this project.
// SPDX-License-Identifier: MIT
//

use plib::{run_test, TestPlan};

fn tr_test(args: &[&str], test_data: &str, expected_output: &str) {
    let str_args: Vec<String> = args.iter().map(|s| String::from(*s)).collect();

    run_test(TestPlan {
        cmd: String::from("tr"),
        args: str_args,
        stdin_data: String::from(test_data),
        expected_out: String::from(expected_output),
        expected_err: String::from(""),
        expected_exit_code: 0,
    });
}

fn nl_test(args: &[&str], test_data: &str, expected_output: &str) {
    let str_args: Vec<String> = args.iter().map(|s| String::from(*s)).collect();

    run_test(TestPlan {
        cmd: String::from("nl"),
        args: str_args,
        stdin_data: String::from(test_data),
        expected_out: String::from(expected_output),
        expected_err: String::from(""),
        expected_exit_code: 0,
    });
}

fn diff_test(args: &[&str], expected_output: &str) {
    let str_args: Vec<String> = args.iter().map(|s| String::from(*s)).collect();

    run_test(TestPlan {
        cmd: String::from("diff"),
        args: str_args,
        stdin_data: String::from(""),
        expected_out: String::from(expected_output),
        expected_err: String::from(""),
        expected_exit_code: 0,
    });
}

fn cut_test(args: &[&str], test_data: &str, expected_output: &str) {
    let str_args: Vec<String> = args.iter().map(|s| String::from(*s)).collect();

    run_test(TestPlan {
        cmd: String::from("cut"),
        args: str_args,
        stdin_data: String::from(test_data),
        expected_out: String::from(expected_output),
        expected_err: String::from(""),
        expected_exit_code: 0,
    });
}

fn sort_test(
    args: &[&str],
    test_data: &str,
    expected_output: &str,
    expected_exit_code: i32,
    expected_err: &str,
) {
    let str_args: Vec<String> = args.iter().map(|s| String::from(*s)).collect();

    run_test(TestPlan {
        cmd: String::from("sort"),
        args: str_args,
        stdin_data: String::from(test_data),
        expected_out: String::from(expected_output),
        expected_err: String::from(expected_err),
        expected_exit_code,
    });
}

fn tail_test(args: &[&str], test_data: &str, expected_output: &str) {
    let str_args: Vec<String> = args.iter().map(|s| String::from(*s)).collect();

    run_test(TestPlan {
        cmd: String::from("tail"),
        args: str_args,
        stdin_data: String::from(test_data),
        expected_out: String::from(expected_output),
        expected_err: String::from(""),
        expected_exit_code: 0,
    });
}

#[test]
fn test_nl_justification() {
    nl_test(&["-n", "ln"], "a", "1     \ta\n");
    nl_test(&["-n", "rn"], "b", "     1\tb\n");
    nl_test(&["-n", "rz"], "c", "000001\tc\n");
}

#[test]
fn test_nl_newlines_at_end() {
    nl_test(&[], "a\n\n", "     1\ta\n       \n");
}

#[test]
fn test_nl_starting_number() {
    nl_test(&["-v", "2"], "a", "     2\ta\n");
}

#[test]
fn test_nl_number_increment() {
    let input = "\\:\\:\\:\nheader\n\\:\\:\nbody\n\\:\nfooter";
    // Without -p, the counter resets on delimiters
    nl_test(
        &["-h", "a", "-f", "a"],
        input,
        "\n     1\theader\n\n     1\tbody\n\n     1\tfooter\n",
    );

    // With -p, the counter increments even when encountering delimiters
    nl_test(
        &["-h", "a", "-f", "a", "-p"],
        input,
        "\n     1\theader\n\n     2\tbody\n\n     3\tfooter\n",
    );

    nl_test(
        &["-h", "a", "-f", "a", "-p", "-i", "2"],
        input,
        "\n     1\theader\n\n     3\tbody\n\n     5\tfooter\n",
    );
}

#[test]
fn test_nl_delimiter() {
    // Single character delimiter should be appended with the default second
    // character, ':'
    nl_test(
        &["-h", "a", "-f", "a", "-d", "?"],
        "?:?:?:\nheader\n?:?:\nbody\n?:\nfooter",
        "\n     1\theader\n\n     1\tbody\n\n     1\tfooter\n",
    );

    nl_test(
        &["-h", "a", "-f", "a", "-d", "?!"],
        "?!?!?!\nheader\n?!?!\nbody\n?!\nfooter",
        "\n     1\theader\n\n     1\tbody\n\n     1\tfooter\n",
    );
}

#[test]
fn test_nl_regex() {
    // NOTE: The implementation has better regex support than the reference.
    // `nl -b p.+ng` would fail to match the words ending with "ng" in the
    // original whereas it would in this Rust implementation. Might be
    // considered a bug?
    nl_test(
        &["-b", "p.*ng"],
        "something\nanything\neverything\ncat\ndog",
        "     1\tsomething\n     2\tanything\n     3\teverything\n       cat\n       dog\n",
    );
}

#[cfg(test)]
mod cut_tests {
    use crate::cut_test;

    #[test]
    fn test_cut_0() {
        cut_test(&["-c", "1-3", "-"], "abcdef", "abc\n");
    }

    #[test]
    fn test_cut_1() {
        cut_test(
            &["-d", ":", "-f", "1,3", "-"],
            "field1:field2:field3",
            "field1:field3\n",
        );
    }

    #[test]
    fn test_cut_2() {
        cut_test(&["-d", ":", "-f", "1,3-", "-"], "a:b:c\n", "a:c\n");
    }

    #[test]
    fn test_cut_3() {
        cut_test(&["-d", ":", "-f", "2-", "-"], "a:b:c\n", "b:c\n");
    }

    #[test]
    fn test_cut_4() {
        cut_test(&["-d", ":", "-f", "4", "-"], "a:b:c\n", "\n");
    }

    #[test]
    fn test_cut_5() {
        cut_test(&["-d", ":", "-f", "4", "-"], "", "");
    }

    #[test]
    fn test_cut_6() {
        cut_test(&["-c", "4", "-"], "123\n", "\n");
    }

    #[test]
    fn test_cut_7() {
        cut_test(&["-c", "4", "-"], "123", "\n");
    }

    #[test]
    fn test_cut_8() {
        cut_test(&["-c", "4", "-"], "123\n1", "\n\n");
    }

    #[test]
    fn test_cut_9() {
        cut_test(&["-c", "4", "-"], "", "");
    }

    #[test]
    fn test_cut_a() {
        cut_test(&["-s", "-d", ":", "-f", "3-", "-"], "a:b:c\n", "c\n");
    }

    #[test]
    fn test_cut_b() {
        cut_test(&["-s", "-d", ":", "-f", "2,3", "-"], "a:b:c\n", "b:c\n");
    }

    #[test]
    fn test_cut_c() {
        cut_test(&["-s", "-d", ":", "-f", "1,3", "-"], "a:b:c\n", "a:c\n");
    }

    #[test]
    fn test_cut_e() {
        cut_test(&["-s", "-d", ":", "-f", "3-", "-"], "a:b:c:\n", "c:\n");
    }

    #[test]
    fn test_cut_f() {
        cut_test(&["-s", "-d", ":", "-f", "3-4", "-"], "a:b:c:\n", "c:\n");
    }

    #[test]
    fn test_cut_h() {
        cut_test(&["-s", "-d", ":", "-f", "2,3", "-"], "abc\n", "");
    }

    #[test]
    fn test_cut_i() {
        cut_test(&["-d", ":", "-f", "1-3", "-"], ":::\n", "::\n");
    }

    #[test]
    fn test_cut_j() {
        cut_test(&["-d", ":", "-f", "1-4", "-"], ":::\n", ":::\n");
    }

    #[test]
    fn test_cut_k() {
        cut_test(&["-d", ":", "-f", "2-3", "-"], ":::\n", ":\n");
    }

    #[test]
    fn test_cut_l() {
        cut_test(&["-d", ":", "-f", "2-4", "-"], ":::\n", "::\n");
    }

    #[test]
    fn test_cut_m() {
        cut_test(&["-s", "-d", ":", "-f", "1-3", "-"], ":::\n", "::\n");
    }

    #[test]
    fn test_cut_n() {
        cut_test(&["-s", "-d", ":", "-f", "1-4", "-"], ":::\n", ":::\n");
    }

    #[test]
    fn test_cut_o() {
        cut_test(&["-s", "-d", ":", "-f", "2-3", "-"], ":::\n", ":\n");
    }

    #[test]
    fn test_cut_p() {
        cut_test(&["-s", "-d", ":", "-f", "2-4", "-"], ":::\n", "::\n");
    }

    #[test]
    fn test_cut_q() {
        cut_test(&["-s", "-d", ":", "-f", "2-4", "-"], ":::\n:\n", "::\n\n");
    }

    #[test]
    fn test_cut_r() {
        cut_test(&["-s", "-d", ":", "-f", "2-4", "-"], ":::\n:1\n", "::\n1\n");
    }

    #[test]
    fn test_cut_s() {
        cut_test(
            &["-s", "-d", ":", "-f", "1-4", "-"],
            ":::\n:a\n",
            ":::\n:a\n",
        );
    }

    #[test]
    fn test_cut_t() {
        cut_test(&["-s", "-d", ":", "-f", "3-", "-"], ":::\n:1\n", ":\n\n");
    }

    #[test]
    fn test_cut_u() {
        cut_test(&["-s", "-f", "3-", "-"], "", "");
    }

    #[test]
    fn test_cut_v() {
        cut_test(&["-f", "3-", "-"], "", "");
    }

    #[test]
    fn test_cut_w() {
        cut_test(&["-b", "1", "-"], "", "");
    }

    #[test]
    fn test_cut_x() {
        cut_test(&["-s", "-d", ":", "-f", "2-4", "-"], ":\n", "\n");
    }

    #[test]
    fn test_cut_newline_1() {
        cut_test(&["-f", "1-", "-"], "a\nb", "a\nb\n");
    }

    #[test]
    fn test_cut_newline_2() {
        cut_test(&["-f", "1-", "-"], "", "");
    }

    #[test]
    fn test_cut_newline_3() {
        cut_test(&["-d", ":", "-f", "1", "-"], "a:1\nb:2\n", "a\nb\n");
    }

    #[test]
    fn test_cut_newline_4() {
        cut_test(&["-d", ":", "-f", "1", "-"], "a:1\nb:2", "a\nb\n");
    }

    #[test]
    fn test_cut_newline_5() {
        cut_test(&["-d", ":", "-f", "2", "-"], "a:1\nb:2\n", "1\n2\n");
    }

    #[test]
    fn test_cut_newline_6() {
        cut_test(&["-d", ":", "-f", "2", "-"], "a:1\nb:2", "1\n2\n");
    }

    #[test]
    fn test_cut_newline_7() {
        cut_test(&["-s", "-d", ":", "-f", "1", "-"], "a:1\nb:2", "a\nb\n");
    }

    #[test]
    fn test_cut_newline_8() {
        cut_test(&["-s", "-d", ":", "-f", "1", "-"], "a:1\nb:2\n", "a\nb\n");
    }

    #[test]
    fn test_cut_newline_9() {
        cut_test(&["-s", "-d", ":", "-f", "1", "-"], "a1\nb2", "");
    }

    #[test]
    fn test_cut_newline_10() {
        cut_test(
            &["-s", "-d", ":", "-f", "1,2", "-"],
            "a:1\nb:2",
            "a:1\nb:2\n",
        );
    }

    #[test]
    fn test_cut_newline_11() {
        cut_test(
            &["-s", "-d", ":", "-f", "1,2", "-"],
            "a:1\nb:2\n",
            "a:1\nb:2\n",
        );
    }

    #[test]
    fn test_cut_newline_12() {
        cut_test(&["-s", "-d", ":", "-f", "1", "-"], "a:1\nb:", "a\nb\n");
    }

    #[test]
    fn test_cut_newline_13() {
        cut_test(&["-d", ":", "-f", "1-", "-"], "a1:\n:", "a1:\n:\n");
    }

    #[test]
    fn test_cut_newline_14() {
        cut_test(&["-d", "\n", "-f", "1-", "-"], "\nb", "\nb\n");
    }

    #[test]
    fn test_out_delim_1() {
        cut_test(&["-d", ":", "-c", "1-3,5-", "-"], "abcdefg\n", "abc:efg\n");
    }

    #[test]
    fn test_out_delim_2() {
        cut_test(
            &["-d", ":", "-c", "1-3,2,5-", "-"],
            "abcdefg\n",
            "abc:efg\n",
        );
    }

    #[test]
    fn test_out_delim_3() {
        cut_test(
            &["-d", ":", "-c", "1-3,2-4,6", "-"],
            "abcdefg\n",
            "abcd:f\n",
        );
    }

    #[test]
    fn test_out_delim_3a() {
        cut_test(
            &["-d", ":", "-c", "1-3,2-4,6-", "-"],
            "abcdefg\n",
            "abcd:fg\n",
        );
    }

    #[test]
    fn test_out_delim_4() {
        cut_test(&["-d", ":", "-c", "4-,2-3", "-"], "abcdefg\n", "bc:defg\n");
    }

    #[test]
    fn test_out_delim_5() {
        cut_test(&["-d", ":", "-c", "2-3,4-", "-"], "abcdefg\n", "bc:defg\n");
    }

    #[test]
    fn test_out_delim_6() {
        cut_test(&["-d", ":", "-c", "2,1-3", "-"], "abc\n", "abc\n");
    }

    #[test]
    fn test_od_abut() {
        cut_test(&["-d", ":", "-b", "1-2,3-4", "-"], "abcd\n", "ab:cd\n");
    }

    #[test]
    fn test_od_overlap() {
        cut_test(&["-d", ":", "-b", "1-2,2", "-"], "abc\n", "ab\n");
    }

    #[test]
    fn test_od_overlap2() {
        cut_test(&["-d", ":", "-b", "1-2,2-", "-"], "abc\n", "abc\n");
    }

    #[test]
    fn test_od_overlap3() {
        cut_test(&["-d", ":", "-b", "1-3,2-", "-"], "abcd\n", "abcd\n");
    }

    #[test]
    fn test_od_overlap4() {
        cut_test(&["-d", ":", "-b", "1-3,2-3", "-"], "abcd\n", "abc\n");
    }

    #[test]
    fn test_od_overlap5() {
        cut_test(&["-d", ":", "-b", "1-3,1-4", "-"], "abcde\n", "abcd\n");
    }
}

#[cfg(test)]
mod sort_tests {
    use crate::sort_test;

    #[test]
    fn test_n1() {
        sort_test(&["-n"], ".01\n0\n", "0\n.01\n", 0, "");
    }

    #[test]
    fn test_n2() {
        sort_test(&["-n"], ".02\n.01\n", ".01\n.02\n", 0, "");
    }

    #[test]
    fn test_n3() {
        sort_test(&["-n"], ".02\n.00\n", ".00\n.02\n", 0, "");
    }

    #[test]
    fn test_n4() {
        sort_test(&["-n"], ".02\n.000\n", ".000\n.02\n", 0, "");
    }

    #[test]
    fn test_n5() {
        sort_test(&["-n"], ".021\n.029\n", ".021\n.029\n", 0, "");
    }

    #[test]
    fn test_n6() {
        sort_test(&["-n"], ".02\n.0*\n", ".0*\n.02\n", 0, "");
    }

    #[test]
    fn test_n7() {
        sort_test(&["-n"], ".02\n.*\n", ".*\n.02\n", 0, "");
    }

    #[test]
    fn test_n8a() {
        sort_test(&["-n", "-k1,1"], ".0a\n.0b\n", ".0a\n.0b\n", 0, "");
    }

    #[test]
    fn test_n8b() {
        sort_test(&["-n", "-k1,1"], ".0b\n.0a\n", ".0b\n.0a\n", 0, "");
    }

    #[test]
    fn test_n9a() {
        sort_test(&["-n", "-k1,1"], ".000a\n.000b\n", ".000a\n.000b\n", 0, "");
    }

    #[test]
    fn test_n9b() {
        sort_test(&["-n", "-k1,1"], ".000b\n.000a\n", ".000b\n.000a\n", 0, "");
    }

    #[test]
    fn test_n10a() {
        sort_test(&["-n", "-k1,1"], ".00a\n.000b\n", ".00a\n.000b\n", 0, "");
    }

    #[test]
    fn test_n10b() {
        sort_test(&["-n", "-k1,1"], ".00b\n.000a\n", ".00b\n.000a\n", 0, "");
    }

    #[test]
    fn test_n11a() {
        sort_test(&["-n", "-k1,1"], ".01a\n.010\n", ".01a\n.010\n", 0, "");
    }

    #[test]
    fn test_n11b() {
        sort_test(&["-n", "-k1,1"], ".010\n.01a\n", ".010\n.01a\n", 0, "");
    }

    #[test]
    fn test_02a() {
        sort_test(&["-c"], "A\nB\nC\n", "", 0, "");
    }

    #[test]
    fn test_02b() {
        sort_test(
            &["-c"],
            "A\nC\nB\n",
            "",
            1,
            "The order of the lines is not correct on line 2:`C`\n",
        );
    }

    #[test]
    fn test_02c() {
        sort_test(&["-c", "-k1,1"], "a\na b\n", "", 0, "");
    }

    #[test]
    fn test_02d() {
        sort_test(&["-C"], "A\nB\nC\n", "", 0, "");
    }

    #[test]
    fn test_02e() {
        sort_test(
            &["-C"],
            "A\nC\nB\n",
            "",
            1,
            "The order of the lines is not correct\n",
        );
    }

    #[test]
    fn test_02m() {
        sort_test(&["-cu"], "A\nA\n", "", 1, "Duplicate key was found! `A`\n");
    }

    #[test]
    fn test_02n() {
        sort_test(&["-cu"], "A\nB\n", "", 0, "");
    }

    #[test]
    fn test_02o() {
        sort_test(
            &["-cu"],
            "A\nB\nB\n",
            "",
            1,
            "Duplicate key was found! `B`\n",
        );
    }

    #[test]
    fn test_02p() {
        sort_test(
            &["-cu"],
            "B\nA\nB\n",
            "",
            1,
            "Duplicate key was found! `B`\n",
        );
    }

    #[test]
    fn test_03a() {
        sort_test(&["-k1", "-"], "B\nA\n", "A\nB\n", 0, "");
    }

    #[test]
    fn test_03b() {
        sort_test(&["-k1,1", "-"], "B\nA\n", "A\nB\n", 0, "");
    }

    #[test]
    fn test_03c() {
        sort_test(&["-k1", "-k2", "-"], "A b\nA a\n", "A a\nA b\n", 0, "");
    }

    #[test]
    fn test_03d() {
        // Fail with a diagnostic when -k specifies field == 0.
        sort_test(&["-k0", "-"], "", "", 1, "the key can't be zero.\n");
    }

    #[test]
    fn test_04a() {
        sort_test(&["-nc", "-"], "2\n11\n", "", 0, "");
    }

    #[test]
    fn test_04b() {
        sort_test(&["-n", "-"], "11\n2\n", "2\n11\n", 0, "");
    }

    #[test]
    fn test_04c() {
        sort_test(&["-k1n", "-"], "11\n2\n", "2\n11\n", 0, "");
    }

    #[test]
    fn test_04d() {
        sort_test(&["-k1", "-"], "11\n2\n", "11\n2\n", 0, "");
    }

    #[test]
    fn test_04e() {
        sort_test(
            &["-k2", "-"],
            "ignored B\nz-ig A\n",
            "z-ig A\nignored B\n",
            0,
            "",
        );
    }

    #[test]
    fn test_05a() {
        sort_test(&["-k1,2", "-"], "A B\nA A\n", "A A\nA B\n", 0, "");
    }

    #[test]
    fn test_05b() {
        sort_test(&["-k1,2", "-"], "A B A\nA A Z\n", "A A Z\nA B A\n", 0, "");
    }

    #[test]
    fn test_05c() {
        sort_test(
            &["-k1", "-k2", "-"],
            "A B A\nA A Z\n",
            "A A Z\nA B A\n",
            0,
            "",
        );
    }

    #[test]
    fn test_05d() {
        sort_test(&["-k2,2", "-"], "A B A\nA A Z\n", "A A Z\nA B A\n", 0, "");
    }

    #[test]
    fn test_05e() {
        sort_test(&["-k2,2", "-"], "A B Z\nA A A\n", "A A A\nA B Z\n", 0, "");
    }

    #[test]
    fn test_05f() {
        sort_test(&["-k2,2", "-"], "A B A\nA A Z\n", "A A Z\nA B A\n", 0, "");
    }

    #[test]
    fn test_07a() {
        sort_test(&["-k2,3", "-"], "9 a b\n7 a a\n", "7 a a\n9 a b\n", 0, "");
    }

    #[test]
    fn test_07b() {
        sort_test(&["-k2,3"], "a a b\nz a a\n", "z a a\na a b\n", 0, "");
    }

    #[test]
    fn test_07c() {
        sort_test(&["-k2,3", "-"], "y k b\nz k a\n", "z k a\ny k b\n", 0, "");
    }

    #[test]
    fn test_07e() {
        // ensure a character position of 0 includes whole field
        sort_test(&["-k2,3.0", "-"], "a a b\nz a a\n", "z a a\na a b\n", 0, "");
    }

    #[test]
    fn test_07f() {
        // ensure fields with end position before start are error
        sort_test(
            &["-n", "-k1.3,1.1", "-"],
            "a 2\nb 1\n",
            "",
            1,
            "keys fields with end position before start!\n",
        );
    }

    #[test]
    fn test_08a() {
        // report an error for '.' without following char spec
        sort_test(
            &["-k", "2.,3", "-"],
            "",
            "",
            1,
            "cannot parse integer from empty string\n",
        );
    }

    #[test]
    fn test_08b() {
        // report an error for ',' without following POS2
        sort_test(
            &["-k", "2,", "-"],
            "",
            "",
            1,
            "cannot parse integer from empty string\n",
        );
    }

    #[test]
    fn test_09b() {
        sort_test(&["-n", "-"], "1e2\n2e1\n", "1e2\n2e1\n", 0, "");
    }

    #[test]
    fn test_09c() {
        sort_test(&["-n", "-"], "2e1\n1e2\n", "1e2\n2e1\n", 0, "");
    }

    #[test]
    fn test_10a() {
        sort_test(
            &["-t", ":", "-k2.2,2.2", "-"],
            ":ba\n:ab\n",
            ":ba\n:ab\n",
            0,
            "",
        );
    }

    #[test]
    fn test_10c() {
        sort_test(
            &["-t", ":", "-k2.2,2.2", "-"],
            ":ab\n:ba\n",
            ":ba\n:ab\n",
            0,
            "",
        );
    }

    #[test]
    fn test_10a0() {
        sort_test(&["-k2.3,2.3", "-"], "z ba\nz ab\n", "z ba\nz ab\n", 0, "");
    }

    #[test]
    fn test_10a1() {
        sort_test(&["-k1.2,1.2", "-"], "ba\nab\n", "ba\nab\n", 0, "");
    }

    #[test]
    fn test_10a2() {
        sort_test(
            &["-b", "-k2.2,2.2", "-"],
            "z ba\nz ab\n",
            "z ba\nz ab\n",
            0,
            "",
        );
    }

    #[test]
    fn test_10e() {
        sort_test(&["-k1.2,1.2", "-"], "ab\nba\n", "ba\nab\n", 0, "");
    }

    #[test]
    fn test_11a() {
        // Exercise bug re using -b to skip trailing blanks.
        sort_test(
            &["-t:", "-k1,1b", "-k2,2", "-"],
            "a\t:a\na :b\n",
            "a\t:a\na :b\n",
            0,
            "",
        );
    }

    #[test]
    fn test_11b() {
        sort_test(
            &["-t:", "-k1,1b", "-k2,2", "-"],
            "a :b\na\t:a\n",
            "a\t:a\na :b\n",
            0,
            "",
        );
    }

    #[test]
    fn test_11c() {
        sort_test(
            &["-t:", "-k2,2b", "-k3,3", "-"],
            "z:a\t:a\na :b\n",
            "z:a\t:a\na :b\n",
            0,
            "",
        );
    }

    #[test]
    fn test_11d() {
        sort_test(
            &["-t:", "-k2,2b", "-k3,3", "-"],
            "z:a :b\na\t:a\n",
            "a\t:a\nz:a :b\n",
            0,
            "",
        );
    }

    #[test]
    fn test_14a() {
        sort_test(
            &["-d", "-u", "-"],
            "mal\nmal-\nmala\n",
            "mal\nmala\n",
            0,
            "",
        );
    }

    #[test]
    fn test_14b() {
        sort_test(
            &["-f", "-d", "-u", "-"],
            "mal\nmal-\nmala\n",
            "mal\nmala\n",
            0,
            "",
        );
    }

    #[test]
    fn test_15a() {
        sort_test(&["-i", "-u", "-"], "a\na\t\n", "a\n", 0, "");
    }

    #[test]
    fn test_15b() {
        sort_test(&["-i", "-u", "-"], "a\n\ta\n", "a\n", 0, "");
    }

    #[test]
    fn test_15c() {
        sort_test(&["-i", "-u", "-"], "a\t\na\n", "a\t\n", 0, "");
    }

    #[test]
    fn test_15d() {
        sort_test(&["-i", "-u", "-"], "\ta\na\n", "\ta\n", 0, "");
    }

    #[test]
    fn test_15e() {
        sort_test(&["-i", "-u", "-"], "a\n\t\t\t\t\ta\t\t\t\t\n", "a\n", 0, "");
    }

    #[test]
    fn test_18a() {
        sort_test(&["-k1.1,1.2n", "-"], " 901\n100\n", " 901\n100\n", 0, "");
    }

    #[test]
    fn test_18b() {
        sort_test(
            &["-b", "-k1.1,1.2n", "-"],
            " 901\n100\n",
            " 901\n100\n",
            0,
            "",
        );
    }

    #[test]
    fn test_18c() {
        sort_test(&["-k1.1,1.2nb", "-"], " 901\n100\n", "100\n 901\n", 0, "");
    }

    #[test]
    fn test_18d() {
        sort_test(&["-k1.1b,1.2n", "-"], " 901\n100\n", " 901\n100\n", 0, "");
    }

    #[test]
    fn test_18e() {
        sort_test(
            &["-nb", "-k1.1,1.2", "-"],
            " 901\n100\n",
            "100\n 901\n",
            0,
            "",
        );
    }

    #[test]
    fn test_18f() {
        sort_test(&["-k1,1b", "-"], "a  y\na z\n", "a  y\na z\n", 0, "");
    }
    #[test]
    fn test_19b() {
        sort_test(
            &["-k1,1", "-k2nr", "-"],
            "b 2\nb 1\nb 3\n",
            "b 3\nb 2\nb 1\n",
            0,
            "",
        );
    }

    #[test]
    fn test_20a() {
        sort_test(
            &["-"],
            "_________U__free\n_________U__malloc\n_________U__abort\n\
         _________U__memcpy\n_________U__memset\n_________U_dyld_stub_binding_helper\n\
         _________U__malloc\n_________U___iob\n_________U__abort\n_________U__fprintf\n",
            "_________U___iob\n_________U__abort\n_________U__abort\n\
         _________U__fprintf\n_________U__free\n_________U__malloc\n\
         _________U__malloc\n_________U__memcpy\n_________U__memset\n\
         _________U_dyld_stub_binding_helper\n",
            0,
            "",
        );
    }

    #[test]
    fn test_21a() {
        sort_test(&["-"], "A\na\n_\n", "A\n_\na\n", 0, "");
    }

    #[test]
    fn test_21b() {
        sort_test(&["-f", "-"], "A\na\n_\n", "A\na\n_\n", 0, "");
    }

    #[test]
    fn test_21c() {
        sort_test(&["-f", "-"], "a\nA\n_\n", "A\na\n_\n", 0, "");
    }

    #[test]
    fn test_21d() {
        sort_test(&["-f", "-"], "_\na\nA\n", "A\na\n_\n", 0, "");
    }

    #[test]
    fn test_21e() {
        sort_test(&["-f", "-"], "a\n_\nA\n", "A\na\n_\n", 0, "");
    }

    #[test]
    fn test_21g() {
        sort_test(&["-f", "-u", "-"], "a\n_\n", "a\n_\n", 0, "");
    }

    #[test]
    fn test_22a() {
        sort_test(
            &["-k2,2fd", "-k1,1r", "-"],
            "3 b\n4 B\n",
            "4 B\n3 b\n",
            0,
            "",
        );
    }

    #[test]
    fn test_neg_nls() {
        sort_test(&["-n", "-"], "-1\n-9\n", "-9\n-1\n", 0, "");
    }

    #[test]
    fn test_nul_nls() {
        sort_test(&["-"], "\0b\n\0a\n", "\0a\n\0b\n", 0, "");
    }

    #[test]
    fn test_use_nl() {
        sort_test(&["-"], "\n\t\n", "\n\t\n", 0, "");
    }

    #[test]
    fn test_files_sort_1() {
        sort_test(
            &["tests/assets/empty_line.txt", "tests/assets/in_uniq"],
            "",
            "\n\n\n\nXX\nXX\nXX\nYY\nYY\nYY\na\na\nb\nb\nc\nd\nd\nd\nline 1\nline 3\n",
            0,
            "",
        );
    }

    #[test]
    fn test_files_sort_2() {
        sort_test(
            &["-n", "tests/assets/in_seq", "tests/assets/test_file.txt"],
            "",
            "1\n1sdfghnm\n2\n2sadsgdhjmf\n3\n3zcxbncvm vbm\n4\n4asdbncv\n5\n5adsbfdgfnfm\n6\n6sdfcvncbmcg\n7zsdgdgfndcgmncg\n8asdbsfdndcgmn\n9sfbdxgfndcgmncgmn\n10dvsd\n11\n12\n13\n14\n15\n16\n17\n",
            0,
            "",
        );
    }
}

#[cfg(test)]
mod tr_tests {
    use crate::tr_test;

    #[test]
    fn test_tr_1() {
        tr_test(&["abcd", "[]*]"], "abcd", "]]]]");
    }

    #[test]
    fn test_tr_2() {
        tr_test(&["abc", "[%*]xyz"], "abc", "xyz");
    }

    #[test]
    fn test_tr_3() {
        tr_test(&["abcd", "xy"], "abcde", "xyyye");
    }

    #[test]
    fn test_tr_4() {
        tr_test(&["abcd", "x[y*]"], "abcde", "xyyye");
    }

    #[test]
    fn test_tr_5() {
        tr_test(&["-s", "a-p", "%[.*]$"], "abcdefghijklmnop", "%.$");
    }

    #[test]
    fn test_tr_6() {
        tr_test(&["-s", "a-p", "[.*]$"], "abcdefghijklmnop", ".$");
    }

    #[test]
    fn test_tr_7() {
        tr_test(&["-s", "a-p", "%[.*]"], "abcdefghijklmnop", "%.");
    }

    #[test]
    fn test_tr_a() {
        tr_test(&["-s", "[a-z]"], "aabbcc", "abc");
    }

    #[test]
    fn test_tr_b() {
        tr_test(&["-s", "[a-c]"], "aabbcc", "abc");
    }

    #[test]
    fn test_tr_c() {
        tr_test(&["-s", "[a-b]"], "aabbcc", "abcc");
    }

    #[test]
    fn test_tr_d() {
        tr_test(&["-s", "[b-c]"], "aabbcc", "aabc");
    }

    #[test]
    fn test_tr_f() {
        tr_test(&["-d", "[=[=]"], "[[[[[[[[]]]]]]]]", "]]]]]]]]");
    }

    #[test]
    fn test_tr_g() {
        tr_test(&["-d", "[=]=]"], "[[[[[[[[]]]]]]]]", "[[[[[[[[");
    }

    #[test]
    fn test_tr_h() {
        tr_test(&["-d", "[:xdigit:]"], "0123456789acbdefABCDEF", "");
    }

    #[test]
    fn test_tr_i() {
        tr_test(
            &["-d", "[:xdigit:]"],
            "w0x1y2z3456789acbdefABCDEFz",
            "wxyzz",
        );
    }

    #[test]
    fn test_tr_j() {
        tr_test(&["-d", "[:digit:]"], "0123456789", "");
    }

    #[test]
    fn test_tr_k() {
        tr_test(&["-d", "[:digit:]"], "a0b1c2d3e4f5g6h7i8j9k", "abcdefghijk");
    }

    #[test]
    fn test_tr_l() {
        tr_test(&["-d", "[:lower:]"], "abcdefghijklmnopqrstuvwxyz", "");
    }

    #[test]
    fn test_tr_m() {
        tr_test(&["-d", "[:upper:]"], "ABCDEFGHIJKLMNOPQRSTUVWXYZ", "");
    }

    #[test]
    fn test_tr_n() {
        tr_test(
            &["-d", "[:lower:][:upper:]"],
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ",
            "",
        );
    }

    #[test]
    fn test_tr_o() {
        tr_test(
            &["-d", "[:alpha:]"],
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ",
            "",
        );
    }

    #[test]
    fn test_tr_p() {
        tr_test(
            &["-d", "[:alnum:]"],
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
            "",
        );
    }

    #[test]
    fn test_tr_q() {
        tr_test(
            &["-d", "[:alnum:]"],
            ".abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.",
            "..",
        );
    }

    #[test]
    fn test_tr_r() {
        tr_test(
            &["-ds", "[:alnum:]", "."],
            ".abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.",
            ".",
        );
    }

    #[test]
    fn test_tr_s() {
        tr_test(
            &["-c", "[:alnum:]", "\n"],
            "The big black fox jumped over the fence.",
            "The\nbig\nblack\nfox\njumped\nover\nthe\nfence\n",
        );
    }

    #[test]
    fn test_tr_t() {
        tr_test(
            &["-c", "[:alnum:]", "[\n*]"],
            "The big black fox jumped over the fence.",
            "The\nbig\nblack\nfox\njumped\nover\nthe\nfence\n",
        );
    }

    #[test]
    fn test_tr_u() {
        tr_test(&["-ds", "b", "a"], "aabbaa", "a");
    }

    #[test]
    fn test_tr_v() {
        tr_test(
            &["-ds", "[:xdigit:]", "Z"],
            "ZZ0123456789acbdefABCDEFZZ",
            "Z",
        );
    }

    #[test]
    fn test_tr_w() {
        tr_test(
            &["-ds", "\u{350}", "\u{345}"],
            "\u{300}\u{301}\u{377}\u{345}\u{345}\u{350}\u{345}",
            "\u{300}\u{301}\u{377}\u{345}",
        );
    }

    #[test]
    fn test_tr_x() {
        tr_test(
            &["-s", "abcdefghijklmn", "[:*016]"],
            "abcdefghijklmnop",
            ":op",
        );
    }

    #[test]
    fn test_tr_y() {
        tr_test(&["-d", "a-z"], "abc $code", " $");
    }

    #[test]
    fn test_tr_z() {
        tr_test(&["-ds", "a-z", "$."], "a.b.c $$$$code\\", ". $\\");
    }

    #[test]
    fn test_tr_range_a_a() {
        tr_test(&["a-a", "z"], "abc", "zbc");
    }

    #[test]
    fn test_tr_upcase() {
        tr_test(&["[:lower:]", "[:upper:]"], "abcxyzABCXYZ", "ABCXYZABCXYZ");
    }

    #[test]
    fn test_tr_dncase() {
        tr_test(&["[:upper:]", "[:lower:]"], "abcxyzABCXYZ", "abcxyzabcxyz");
    }

    #[test]
    fn test_tr_rep_2() {
        tr_test(&["a[b*512]c", "1[x*]2"], "abc", "1x2");
    }

    #[test]
    fn test_tr_rep_3() {
        tr_test(&["a[b*513]c", "1[x*]2"], "abc", "1x2");
    }

    #[test]
    fn test_tr_o_rep_2() {
        tr_test(&["[b*010]cd", "[a*7]BC[x*]"], "bcd", "BCx");
    }

    #[test]
    fn test_tr_ross_1a() {
        tr_test(&["-cs", "[:upper:]", "[X*]"], "AMZamz123.-+AMZ", "AMZXAMZ");
    }

    #[test]
    fn test_tr_ross_1b() {
        tr_test(&["-cs", "[:upper:][:digit:]", "[Z*]"], "", "");
    }

    #[test]
    fn test_tr_ross_2() {
        tr_test(&["-dcs", "[:lower:]", "n-rs-z"], "amzAMZ123.-+amz", "amzam");
    }

    #[test]
    fn test_tr_ross_3() {
        tr_test(
            &["-ds", "[:xdigit:]", "[:alnum:]"],
            ".ZABCDEFzabcdefg.0123456788899.GG",
            ".Zzg..G",
        );
    }

    #[test]
    fn test_tr_ross_4() {
        tr_test(&["-dcs", "[:alnum:]", "[:digit:]"], "", "");
    }

    #[test]
    fn test_tr_ross_5() {
        tr_test(&["-dc", "[:lower:]"], "", "");
    }

    #[test]
    fn test_tr_ross_6() {
        tr_test(&["-dc", "[:upper:]"], "", "");
    }

    #[test]
    fn test_tr_repeat_0() {
        tr_test(&["abc", "[b*0]"], "abcd", "bbbd");
    }

    #[test]
    fn test_tr_repeat_zeros() {
        tr_test(&["abc", "[b*00000000000000000000]"], "abcd", "bbbd");
    }

    #[test]
    fn test_tr_repeat_compl() {
        tr_test(&["-c", "[a*65536]\n", "[b*]"], "abcd", "abbb");
    }

    #[test]
    fn test_tr_repeat_xc() {
        tr_test(&["-C", "[a*65536]\n", "[b*]"], "abcd", "abbb");
    }

    #[test]
    fn test_tr_no_abort_1() {
        tr_test(&["-c", "a", "[b*256]"], "abc", "abb");
    }
}

#[cfg(test)]
mod diff_tests {
    use crate::diff_test;
    use std::{path::PathBuf, process::Stdio};

    fn diff_base_path() -> PathBuf {
        PathBuf::from("tests").join("diff")
    }

    fn f1_txt_path() -> String {
        diff_base_path()
            .join("f1.txt")
            .to_str()
            .expect("Could not unwrap f1_txt_path")
            .to_string()
    }

    fn f2_txt_path() -> String {
        diff_base_path()
            .join("f2.txt")
            .to_str()
            .expect("Could not unwrap f2_txt_path")
            .to_string()
    }

    fn f1_dir_path() -> String {
        diff_base_path()
            .join("f1")
            .to_str()
            .expect("Could not unwrap f1_dir_path")
            .to_string()
    }

    fn f2_dir_path() -> String {
        diff_base_path()
            .join("f2")
            .to_str()
            .expect("Could not unwrap f2_dir_path")
            .to_string()
    }

    fn f1_txt_with_eol_spaces_path() -> String {
        diff_base_path()
            .join("f1_with_eol_spaces.txt")
            .to_str()
            .expect("Could not unwrap f1_txt_with_eol_spaces_path")
            .to_string()
    }

    struct DiffTestHelper {
        pub key: String,
        content: String,
        file1_path: String,
        file2_path: String,
    }

    impl DiffTestHelper {
        fn new(options: &str, file1_path: String, file2_path: String, key: String) -> Self {
            let args = format!(
                "run --release --bin diff --{} {} {}",
                options, file1_path, file2_path
            );

            let args_list = args.split(' ').collect::<Vec<&str>>();

            let output = std::process::Command::new("cargo")
                .args(args_list)
                // .stdout(output_file)
                .stdout(Stdio::piped())
                .output()
                .expect("Could not run cargo command!");

            let content =
                String::from_utf8(output.stdout).expect("Failed to read output of Command!");

            Self {
                key,
                file1_path,
                file2_path,
                content,
            }
        }

        fn content(&self) -> &str {
            &self.content
        }

        fn file1_path(&self) -> &str {
            &self.file1_path
        }

        fn file2_path(&self) -> &str {
            &self.file2_path
        }
    }

    static mut DIFF_TEST_INPUT: Vec<DiffTestHelper> = vec![];

    fn input_by_key(key: &str) -> &DiffTestHelper {
        unsafe {
            DIFF_TEST_INPUT
                .iter()
                .filter(|data| data.key == key)
                .nth(0)
                .unwrap()
        }
    }

    #[ctor::ctor]
    fn diff_tests_setup() {
        let diff_test_helper_init_data = [
            ("", f1_txt_path(), f2_txt_path(), "test_diff_normal"),
            (" -c", f1_txt_path(), f2_txt_path(), "test_diff_context3"),
            (" -C 1", f1_txt_path(), f2_txt_path(), "test_diff_context1"),
            (
                " -C 10",
                f1_txt_path(),
                f2_txt_path(),
                "test_diff_context10",
            ),
            (" -e", f1_txt_path(), f2_txt_path(), "test_diff_edit_script"),
            (
                " -f",
                f1_txt_path(),
                f2_txt_path(),
                "test_diff_forward_edit_script",
            ),
            (" -u", f1_txt_path(), f2_txt_path(), "test_diff_unified3"),
            (" -U 0", f1_txt_path(), f2_txt_path(), "test_diff_unified0"),
            (
                " -U 10",
                f1_txt_path(),
                f2_txt_path(),
                "test_diff_unified10",
            ),
            ("", f1_txt_path(), f2_dir_path(), "test_diff_file_directory"),
            ("", f1_dir_path(), f2_dir_path(), "test_diff_directories"),
            (
                " -r",
                f1_dir_path(),
                f2_dir_path(),
                "test_diff_directories_recursive",
            ),
            (
                " -r -c",
                f1_dir_path(),
                f2_dir_path(),
                "test_diff_directories_recursive_context",
            ),
            (
                " -r -e",
                f1_dir_path(),
                f2_dir_path(),
                "test_diff_directories_recursive_edit_script",
            ),
            (
                " -r -f",
                f1_dir_path(),
                f2_dir_path(),
                "test_diff_directories_recursive_forward_edit_script",
            ),
            (
                " -r -u",
                f1_dir_path(),
                f2_dir_path(),
                "test_diff_directories_recursive_unified",
            ),
            (
                "",
                f1_txt_path(),
                f1_txt_with_eol_spaces_path(),
                "test_diff_counting_eol_spaces",
            ),
            (
                " -b",
                f1_txt_path(),
                f1_txt_with_eol_spaces_path(),
                "test_diff_ignoring_eol_spaces",
            ),
            (
                " --label F1 --label2 F2 -u",
                f1_txt_path(),
                f1_txt_with_eol_spaces_path(),
                "test_diff_unified_two_labels",
            ),
        ];

        for row in diff_test_helper_init_data {
            unsafe {
                DIFF_TEST_INPUT.push(DiffTestHelper::new(row.0, row.1, row.2, row.3.to_string()))
            }
        }
    }

    #[test]
    fn test_diff_normal() {
        let data = input_by_key("test_diff_normal");
        diff_test(&[data.file1_path(), data.file2_path()], data.content());
    }

    #[test]
    fn test_diff_context3() {
        let data = input_by_key("test_diff_context3");

        diff_test(
            &["-c", data.file1_path(), data.file2_path()],
            data.content(),
        );
    }

    #[test]
    fn test_diff_context1() {
        let data = input_by_key("test_diff_context1");

        diff_test(
            &["-C", "1", data.file1_path(), data.file2_path()],
            data.content(),
        );
    }

    #[test]
    fn test_diff_context10() {
        let data = input_by_key("test_diff_context10");

        diff_test(
            &["-C", "10", data.file1_path(), data.file2_path()],
            data.content(),
        );
    }

    #[test]
    fn test_diff_edit_script() {
        let data = input_by_key("test_diff_edit_script");

        diff_test(
            &["-e", data.file1_path(), data.file2_path()],
            data.content(),
        );
    }

    #[test]
    fn test_diff_forward_edit_script() {
        let data = input_by_key("test_diff_forward_edit_script");

        diff_test(
            &["-f", data.file1_path(), data.file2_path()],
            data.content(),
        );
    }

    #[test]
    fn test_diff_unified3() {
        let data = input_by_key("test_diff_unified3");

        diff_test(
            &["-u", data.file1_path(), data.file2_path()],
            data.content(),
        );
    }

    #[test]
    fn test_diff_unified0() {
        let data = input_by_key("test_diff_unified0");

        diff_test(
            &["-U", "0", data.file1_path(), data.file2_path()],
            data.content(),
        );
    }

    #[test]
    fn test_diff_unified10() {
        let data = input_by_key("test_diff_unified10");

        diff_test(
            &["-U", "10", data.file1_path(), data.file2_path()],
            data.content(),
        );
    }

    #[test]
    fn test_diff_file_directory() {
        let data = input_by_key("test_diff_file_directory");
        diff_test(&[data.file1_path(), data.file2_path()], data.content());
    }

    #[test]
    fn test_diff_directories() {
        let data = input_by_key("test_diff_directories");
        diff_test(&[data.file1_path(), data.file2_path()], data.content());
    }

    #[test]
    fn test_diff_directories_recursive() {
        let data = input_by_key("test_diff_directories_recursive");

        diff_test(
            &["-r", data.file1_path(), data.file2_path()],
            data.content(),
        );
    }

    #[test]
    fn test_diff_directories_recursive_context() {
        let data = input_by_key("test_diff_directories_recursive_context");

        diff_test(
            &["-r", "-c", data.file1_path(), data.file2_path()],
            data.content(),
        );
    }

    #[test]
    fn test_diff_directories_recursive_edit_script() {
        let data = input_by_key("test_diff_directories_recursive_edit_script");

        diff_test(
            &["-r", "-e", data.file1_path(), data.file2_path()],
            data.content(),
        );
    }

    #[test]
    fn test_diff_directories_recursive_forward_edit_script() {
        let data = input_by_key("test_diff_directories_recursive_forward_edit_script");

        diff_test(
            &["-r", "-f", data.file1_path(), data.file2_path()],
            data.content(),
        );
    }

    #[test]
    fn test_diff_directories_recursive_unified() {
        let data = input_by_key("test_diff_directories_recursive_unified");

        diff_test(
            &["-r", "-u", data.file1_path(), data.file2_path()],
            data.content(),
        );
    }

    #[test]
    fn test_diff_counting_eol_spaces() {
        let data = input_by_key("test_diff_counting_eol_spaces");
        diff_test(&[data.file1_path(), data.file2_path()], data.content());
    }

    #[test]
    fn test_diff_ignoring_eol_spaces() {
        let data = input_by_key("test_diff_ignoring_eol_spaces");

        diff_test(
            &["-b", data.file1_path(), data.file2_path()],
            data.content(),
        );
    }

    #[test]
    fn test_diff_unified_two_labels() {
        let data = input_by_key("test_diff_unified_two_labels");

        diff_test(
            &[
                "--label",
                "F1",
                "--label2",
                "F2",
                "-u",
                data.file1_path(),
                data.file2_path(),
            ],
            data.content(),
        );
    }
}

#[cfg(test)]
mod tail_tests {
    use crate::tail_test;

    #[test]
    fn test_tail() {
        tail_test(&["-n2"], "a\nb\nc\n", "b\nc\n");
    }

    #[test]
    fn test_tail_1() {
        tail_test(&["-c+2"], "abcd", "bcd");
    }

    #[test]
    fn test_tail_2() {
        tail_test(&["-c+8"], "abcd", "");
    }

    #[test]
    fn test_tail_3() {
        tail_test(&["-c-1"], "abcd", "d");
    }

    #[test]
    fn test_tail_4() {
        tail_test(&["-c-9"], "abcd", "abcd");
    }

    #[test]
    fn test_tail_5() {
        tail_test(
            &["-c-12"],
            &("x".to_string() + &"y".repeat(12) + "z"),
            &("y".repeat(11) + "z"),
        );
    }

    #[test]
    fn test_tail_6() {
        tail_test(&["-n-1"], "x\n", "x\n");
    }

    #[test]
    fn test_tail_7() {
        tail_test(&["-n-1"], "x\ny\n", "y\n");
    }

    #[test]
    fn test_tail_8() {
        tail_test(&["-n-1"], "x\ny\n", "y\n");
    }

    #[test]
    fn test_tail_9() {
        tail_test(&["-n+1"], "x\ny\n", "x\ny\n");
    }

    #[test]
    fn test_tail_10() {
        tail_test(&["-n+2"], "x\ny\n", "y\n");
    }

    #[test]
    fn test_tail_11() {
        tail_test(
            &["-c+10"],
            &("x".to_string() + &"y".repeat(10) + "z\n"),
            "yyz\n",
        );
    }

    #[test]
    fn test_tail_12() {
        tail_test(
            &["-n+10"],
            &("x\n".to_string() + &"y\n".repeat(10) + "z\n"),
            "y\ny\nz\n",
        );
    }

    #[test]
    fn test_tail_13() {
        tail_test(
            &["-n-10"],
            &("x\n".to_string() + &"y\n".repeat(10) + "z\n"),
            &("y\n".repeat(9) + "z\n"),
        );
    }

    #[test]
    fn test_tail_14() {
        let input = &("x\n".repeat(512 * 10 / 2 + 1));
        let expected_output = &("x\n".repeat(10));
        tail_test(&["-n-10"], input, expected_output);
    }

    #[test]
    fn test_tail_15() {
        tail_test(&["-c2"], "abcd\n", "d\n");
    }

    #[test]
    fn test_tail_16() {
        tail_test(
            &["-n-10"],
            &("x\n".to_string() + &"y\n".repeat(10) + "z\n"),
            &("y\n".repeat(9) + "z\n"),
        );
    }

    #[test]
    fn test_tail_17() {
        tail_test(
            &["-n+10"],
            &("x\n".to_string() + &"y\n".repeat(10) + "z\n"),
            "y\ny\nz\n",
        );
    }

    #[test]
    fn test_tail_18() {
        tail_test(&["-n+0"], &("y\n".repeat(5)), &("y\n".repeat(5)));
    }

    #[test]
    fn test_tail_19() {
        tail_test(&["-n+1"], &("y\n".repeat(5)), &("y\n".repeat(5)));
    }

    #[test]
    fn test_tail_20() {
        tail_test(&["-n-1"], &("y\n".repeat(5)), "y\n");
    }
}
