module top;
  function automatic logic signed [63:0] fs64(input int a); return -a; endfunction
  if (fs64(4) ==? 4'sb1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
