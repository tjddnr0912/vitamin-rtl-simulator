module top;
  typedef int unsigned u32_t;
  localparam u32_t PUT = 32'hFFFF_FFFC;
  if (PUT ==? 4'sb1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
