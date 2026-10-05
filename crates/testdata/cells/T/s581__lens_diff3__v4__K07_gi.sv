module top;
  localparam logic [39:0] PA [2] = '{40'h10_0000_000C, 40'h0};
  if ($size(PA) ==? 2'b1?) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
