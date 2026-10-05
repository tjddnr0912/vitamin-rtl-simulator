module top;
  localparam logic [64:0] P65 = 65'hC;
  if ((P65 >> 1) ==? 4'b0?10) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
