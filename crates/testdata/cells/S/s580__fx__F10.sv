`timescale 1ns/1ns
package p; function automatic logic pf(input logic [3:0] a); return a inside {4'b1?00}; endfunction endpackage
module t;
  logic [3:0] v; logic r; wire wf; wire [7:0] w8;
  function automatic logic fa(input logic [3:0] a); return a inside {4'b1?00}; endfunction
  function logic fs(input logic [3:0] a); fs = a inside {4'b1?00}; endfunction
  function logic fsx(input logic [3:0] a); fsx = a inside {4'b1?00, 'x}; endfunction
  task automatic tk(input logic [3:0] a, output logic o); o = a inside {4'b1?00}; endtask
  assign wf = fa(v);
  assign w8 = fsx(v) + fs(v);
  initial begin
    v = 4'b1100; tk(v, r); #1
    $display("frame %b inlined %b task %b pkg %b cont-frame %b cont-inlined %h", fa(v), fs(v), r, p::pf(v), wf, w8);
    v = 4'b0110; tk(v, r); #1
    $display("frame %b inlined %b task %b pkg %b cont-frame %b cont-inlined %h", fa(v), fs(v), r, p::pf(v), wf, w8);
    $finish;
  end
endmodule
