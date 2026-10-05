module top;
  localparam integer unsigned PIU = 32'hFFFF_FFFC;
  if (PIU ==? 4'sb1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
