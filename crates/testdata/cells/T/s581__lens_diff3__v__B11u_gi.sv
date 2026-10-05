module top;
  function automatic logic [64:0] f65(input int a); return a; endfunction
  if (f65(12) ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
