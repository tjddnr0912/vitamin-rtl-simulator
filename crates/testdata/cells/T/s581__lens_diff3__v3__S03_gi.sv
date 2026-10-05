module top;
  localparam byte unsigned PBU = 8'hFC;
  if (PBU ==? 4'sb1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
