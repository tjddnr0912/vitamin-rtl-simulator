module top;
  localparam logic [67:0] P68F = 68'hC;
  if ($unsigned(P68F) ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
