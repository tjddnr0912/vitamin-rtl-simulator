`ifdef IV
 `define IN(a,b)  ((a) ==? b)
`else
 `define IN(a,b)  ((a) inside {b})
`endif
module t;
  longint l; logic signed [7:0] s8; logic r1, r2;
`ifndef IV
  always_comb begin
    case (s8 >>> 1) inside
      8'sb1100_000?: r1 = 1;
      default: r1 = 0;
    endcase
  end
  always_comb begin
    case (s8 >>> 2) inside
      8'b1110_000?, 8'sb0010_000?: r2 = 1;
      default: r2 = 0;
    endcase
  end
`endif
  initial begin
    l = -256; s8 = -128;
    #1;
    $display("C01 %b", ((l / 3) ==? 'sb?1011));
    $display("C02 %b", ((l / 3) !=? 'sb?1011));
    $display("C03 %b", `IN(l / 3, 'sb?1011));
`ifndef IV
    $display("C04 %b", r1);
    $display("C05 %b", r2);
`endif
    #1 $finish;
  end
endmodule
