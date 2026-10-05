module top;
  localparam logic signed [63:0] S64P = 64'sd12;
  if (S64P ==? 4'sb1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
