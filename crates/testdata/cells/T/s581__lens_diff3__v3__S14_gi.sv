module top;
  localparam logic [39:0] P40 = 40'h10_0000_000C;
  if ($clog2(P40) ==? 6'sb10_0?01) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
