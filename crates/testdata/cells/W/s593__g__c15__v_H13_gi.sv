module top;
  localparam logic [63:0] P64H = 64'h8000_0000_0000_000C;
  if (P64H ==? 'bx100) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
