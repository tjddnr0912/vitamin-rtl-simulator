module top;
  localparam logic [1:0][63:0] PQ = {64'hC, 64'h8000_0000_0000_000C};
  if (PQ[1] ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
