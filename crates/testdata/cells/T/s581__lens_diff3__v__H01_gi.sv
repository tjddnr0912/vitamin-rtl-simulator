module top;
  localparam logic [63:0] P64 = 64'hC;
  if (P64 ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
