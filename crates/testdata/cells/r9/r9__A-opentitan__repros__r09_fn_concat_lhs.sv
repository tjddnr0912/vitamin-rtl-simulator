module t;
  function automatic logic [3:0] f(logic [7:0] x);
    logic [3:0] hi; logic [3:0] lo;
    {hi, lo} = x + 8'h11;
    return hi ^ lo;
  endfunction
  logic [3:0] o; logic [7:0] a;
  assign o = f(a);
  initial begin a = 8'h5A; #1 $display("A o=%h", o); $finish; end
endmodule
