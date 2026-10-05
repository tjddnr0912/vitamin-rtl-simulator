// P4a: S5 — consumers of a DECLINED x/z element (fold_region declines, const_compare_special not admitted): range bound
module t;
`ifndef NO_INSIDE
  logic [(4'b1100 inside {4'sb1?00}) : 0] rbs;            // signed pattern: IEEE 1 -> [1:0] -> 2
  logic [(4'b1100 inside {'b1?00}) : 0] rbu;              // unsized pattern: 2
  logic [(4'b1100 inside {4'b1?00, 4'b0?00}) : 0] rb2;    // two x/z elements: 2
  logic [(4'b1100 inside {4'b1?00}) : 0] rb1;             // admitted, no overflow: 2
`endif
  logic [(4'b1100 ==? 4'sb1?00) : 0] rbsq;                // 2
  logic [(4'b1100 ==? 'b1?00) : 0] rbuq;                  // 2
  logic [((4'b1100 ==? 4'b1?00) || (4'b1100 ==? 4'b0?00)) : 0] rb2q; // 2
  initial begin
`ifndef NO_INSIDE
    $display("rbs=%0d rbu=%0d rb2=%0d rb1=%0d", $bits(rbs), $bits(rbu), $bits(rb2), $bits(rb1));
`endif
    $display("rbsq=%0d rbuq=%0d rb2q=%0d", $bits(rbsq), $bits(rbuq), $bits(rb2q));
    #1 $finish;
  end
endmodule
