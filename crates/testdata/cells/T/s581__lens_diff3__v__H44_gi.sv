module top;
  localparam logic [64:0] P65 = 65'hC;
  if (P65 inside {4'b1?00, 4'b0?11}) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
