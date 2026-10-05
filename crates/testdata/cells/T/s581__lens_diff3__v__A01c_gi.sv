module top;
  localparam logic [39:0] P40F = 40'h0C;
  if ($unsigned(P40F) ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
