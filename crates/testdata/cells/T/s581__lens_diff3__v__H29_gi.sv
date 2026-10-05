module top;
  localparam logic [63:0] P64 = 64'hC;
  if ((P64 << 1) ==? 5'b1?000) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
