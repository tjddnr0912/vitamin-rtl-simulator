module top;
  function automatic logic zf(input logic v, input integer id);
    $display("zf%0d t=%0t v=%b", id, $time, v);
    zf = (v === 1'bz) ? 1'b0 : ~v;
  endfunction
  wire q, qb;
  assign qb = zf(q, 2);
  assign q = zf(qb, 1);
  initial #1 begin $display("t1 q=%b qb=%b", q, qb); $finish; end
  initial #10 $finish;
endmodule
