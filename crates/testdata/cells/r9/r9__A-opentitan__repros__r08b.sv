module t;
  function automatic bit chk();
    bit u [2];
    u = '{default: 1'b0};
    return u[0];
  endfunction
  logic o;
  initial begin o = chk(); #1 $display("A o=%b", o); $finish; end
endmodule
