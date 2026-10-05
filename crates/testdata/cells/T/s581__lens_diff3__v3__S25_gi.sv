module top;
  localparam int unsigned PU = 32'hFFFF_FFFC;
  if (PU ==? 'sbx100) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
