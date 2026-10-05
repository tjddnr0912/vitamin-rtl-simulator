module top;
  localparam logic [67:0] P68H = 68'h1_0000_0000_0000_000C;
  if ($unsigned(P68H) ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
