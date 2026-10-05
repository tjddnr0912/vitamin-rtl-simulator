module top;
  localparam logic [64:0] P65H = 65'h0_8000_0000_0000_000C;
  if (P65H !=? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
