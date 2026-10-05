module top;
  localparam logic [3:0] PX = 4'b1x00;
  if (PX ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
