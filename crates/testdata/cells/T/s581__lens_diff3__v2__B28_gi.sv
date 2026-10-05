module top;
  localparam string SP = "A";
  if (SP ==? 8'b0100_0?01) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
